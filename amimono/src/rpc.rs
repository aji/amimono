//! Subsystem for building components with an RPC interface.
//!
//! It's recommended to use this module via the [`rpc_ops!`][crate::rpc_ops]
//! macro. When using that macro, it's rarely necessary to use any of the
//! definitions in this module directly, however they are documented for the
//! sake of completeness.

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, LazyLock, Mutex},
};

use futures::{
    FutureExt,
    future::{BoxFuture, Shared},
};
use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};

use crate::{
    component::{Component, ComponentKind, Location},
    runtime,
};

/// The port used for the RPC HTTP server
pub const PORT: u16 = 9099;

pub type RpcResult<T> = Result<T, RpcError>;

/// An error when making an RPC call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RpcError {
    /// A miscellaneous error with an unstructured string message. These should
    /// generally be assumed to be unrecoverable.
    Misc(String),
}

impl axum::response::IntoResponse for RpcError {
    fn into_response(self) -> axum::response::Response {
        let res = (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(self),
        );
        res.into_response()
    }
}

impl<S: AsRef<str>> From<S> for RpcError {
    fn from(s: S) -> Self {
        RpcError::Misc(s.as_ref().to_owned())
    }
}

/// A type that can be used as an RPC request or response.
///
/// Message types created by the [`rpc_ops!`][crate::rpc_ops] macro are
/// automatically given an `RpcMessage` impl.
pub trait RpcMessage: Serialize + for<'a> Deserialize<'a> + Send + 'static {
    fn verb(&self) -> &'static str;
}

/// A type representing an RPC component.
///
/// Types with an `RpcComponentKind` impl get an automatic `ComponentKind` impl
/// as well.
pub trait RpcComponentKind: 'static {
    type Request: RpcMessage;
    type Response: RpcMessage;

    const LABEL: &'static str;
}

impl<T: RpcComponentKind> ComponentKind for T {
    type Instance = Arc<dyn RpcInstance<Self>>;

    const LABEL: &'static str = T::LABEL;
    const PORTS: &'static [u16] = &[PORT];
}

/// An RPC component's instance, used as a trait object.
pub trait RpcInstance<T: RpcComponentKind>: Send + Sync {
    fn handle(&'_ self, q: T::Request) -> BoxFuture<'_, RpcResult<T::Response>>;
}

/// A type implementing an RPC component.
///
/// Types with an `RpcComponent` impl get automatic `RpcInstance` and
/// `Component` impls as well.
pub trait RpcComponent: Send + Sync + 'static {
    type Kind: RpcComponentKind;

    fn start() -> impl Future<Output = Self> + Send;

    fn handle(
        &self,
        q: <Self::Kind as RpcComponentKind>::Request,
    ) -> impl Future<Output = RpcResult<<Self::Kind as RpcComponentKind>::Response>> + Send;
}

impl<T: RpcComponent> RpcInstance<T::Kind> for T {
    fn handle(
        &'_ self,
        q: <T::Kind as RpcComponentKind>::Request,
    ) -> BoxFuture<'_, RpcResult<<T::Kind as RpcComponentKind>::Response>> {
        Box::pin(RpcComponent::handle(self, q))
    }
}

impl<T: RpcComponent> Component for T {
    type Kind = T::Kind;

    fn main<F>(set_instance: F) -> impl Future<Output = ()> + Send
    where
        F: FnOnce(<Self::Kind as ComponentKind>::Instance) -> BoxFuture<'static, ()> + Send,
    {
        Box::pin(async {
            let instance = Arc::new(T::start().await);
            let erased = Arc::new(DefaultHttpInstance::<T::Kind>(instance.clone()));
            HTTP_HANDLERS
                .lock()
                .unwrap()
                .insert(<Self::Kind as ComponentKind>::LABEL, erased);
            set_instance(instance.clone()).await;
            HTTP_SERVER.clone().await;
        })
    }
}

/// A client for making requests to an RPC component.
///
/// Cloning values of this type will result in clients that share resources
/// such as connection pools.
///
/// The `Client` struct defined by the [`rpc_ops!`][crate::rpc_ops] macro is a
/// thin wrapper around this type.
pub struct RpcClient<T: RpcComponentKind> {
    instance: Option<LazyLock<Arc<dyn RpcInstance<T>>>>,
}

impl<T: RpcComponentKind> Clone for RpcClient<T> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

fn init_client_instance<T: RpcComponentKind>() -> Arc<dyn RpcInstance<T>> {
    T::instance().expect("no instance")
}

impl<R: RpcComponentKind> RpcClient<R> {
    /// Create a new client for a particular `Rpc` impl. If an existing client
    /// can be cloned, that should be preferred, as it will result in resources
    /// being shared between the clients.
    pub fn new() -> RpcClient<R> {
        let instance = if R::is_local() {
            Some(LazyLock::new(
                init_client_instance::<R> as fn() -> Arc<dyn RpcInstance<R>>,
            ))
        } else {
            None
        };
        Self { instance }
    }

    /// Send a request. If the target `Rpc` impl belongs to a component that is
    /// running in the same process, this will result in the target handler
    /// being invoked directly.
    pub async fn call(&self, q: R::Request) -> RpcResult<R::Response> {
        match &self.instance {
            Some(inner) => inner.handle(q).await,
            None => http_call::<R>(q).await,
        }
    }

    /// Send a request to a specific location. If the target location is the
    /// current location, this will be sent in-process. Otherwise, it will be sent
    /// over HTTP.
    pub async fn call_at(&self, loc: Location, q: R::Request) -> RpcResult<R::Response> {
        // TODO: not 100% sure why this box is needed but the futures types are
        // too complicated for rustc rpc_ops! handlers for some reason and I'm
        // choosing not to dig into it right now.
        let block: BoxFuture<'_, RpcResult<R::Response>> = Box::pin(async {
            if R::is_local()
                && R::myself().await.as_ref().ok() == Some(&loc)
                && self.instance.is_some()
            {
                self.instance.as_ref().unwrap().handle(q).await
            } else {
                http_call_at::<R>(loc, q).await
            }
        });
        block.await
    }
}

// Everything below this line is HTTP implementation details
// -----------------------------------------------------------------------------

trait HttpInstance: Send + Sync + 'static {
    fn handle_json<'h, 'q, 'f>(&'h self, q: &'q [u8]) -> BoxFuture<'f, RpcResult<Vec<u8>>>
    where
        'h: 'f,
        'q: 'f;
}

struct DefaultHttpInstance<T: RpcComponentKind>(<T as ComponentKind>::Instance);

impl<T: RpcComponentKind> HttpInstance for DefaultHttpInstance<T> {
    fn handle_json<'h, 'q, 'f>(&'h self, q: &'q [u8]) -> BoxFuture<'f, RpcResult<Vec<u8>>>
    where
        'h: 'f,
        'q: 'f,
    {
        Box::pin(async {
            let q = match serde_json::from_slice::<T::Request>(q) {
                Ok(q) => q,
                Err(e) => Err(RpcError::Misc(format!("request parse error: {e}")))?,
            };
            let a = self.0.handle(q).await?;
            let res = match serde_json::to_vec(&a) {
                Ok(res) => res,
                Err(e) => Err(RpcError::Misc(format!("serialization failed: {e}")))?,
            };
            Ok(res)
        })
    }
}

static HTTP_SERVER: LazyLock<Shared<BoxFuture<'static, ()>>> = LazyLock::new(|| {
    let fut = rpc_http_server().boxed().shared();
    tokio::task::spawn(fut.clone());
    fut
});

static HTTP_HANDLERS: LazyLock<Mutex<HashMap<&'static str, Arc<dyn HttpInstance>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    log::debug!("created global reqwest HTTP client");
    reqwest::Client::new()
});

async fn rpc_http_server() {
    let app = axum::Router::new().route(
        "/rpc/{label}",
        axum::routing::post(
            async |axum::extract::Path(label): axum::extract::Path<String>,
                   body: axum::body::Bytes| {
                let bytes = body.to_vec();
                let handler = {
                    let lock = HTTP_HANDLERS.lock().unwrap();
                    lock.get(label.as_str()).cloned()
                };
                match handler {
                    Some(h) => h.handle_json(&bytes).await,
                    None => Err(RpcError::Misc(format!("no handler for {label}"))),
                }
            },
        ),
    );

    let addr: SocketAddr = runtime::to_addr(PORT);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    log::info!("rpc server listening on {:?}", addr);
    axum::serve(listener, app).await.unwrap();
}

async fn http_call<R: RpcComponentKind>(q: R::Request) -> RpcResult<R::Response> {
    let loc = match R::discover_running().await {
        Ok(locs) => match locs.choose(&mut rand::rng()) {
            Some(x) => x.clone(),
            None => return Err(RpcError::Misc(format!("discovery endpoints empty"))),
        },
        Err(e) => return Err(RpcError::Misc(format!("could not discover endpoint: {e}"))),
    };
    http_call_at::<R>(loc, q).await
}

async fn http_call_at<R: RpcComponentKind>(loc: Location, q: R::Request) -> RpcResult<R::Response> {
    let label = R::LABEL;
    let hostname = match loc {
        Location::Ephemeral(s) => s,
        Location::Stable(s) => s,
    };
    let url = format!("http://{}:{}/rpc/{}", hostname, PORT, label);
    log::debug!("outgoing RPC: {} -> {}", label, url);
    let resp = HTTP_CLIENT
        .post(&url)
        .json(&q)
        .send()
        .await
        .map_err(|e| RpcError::Misc(format!("failed to send request: {}", e)))?;
    let status = resp.status();
    if !status.is_success() {
        let msg = resp
            .json::<RpcError>()
            .await
            .unwrap_or_else(|e| RpcError::Misc(format!("failed to get request body: {}", e)));
        return Err(msg);
    }
    let resp_msg = resp
        .json::<R::Response>()
        .await
        .map_err(|e| RpcError::Misc(format!("failed to parse response: {}", e)))?;
    Ok(resp_msg)
}
