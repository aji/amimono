//! Subsystem for building components with an RPC interface.
//!
//! It's recommended to use this module via the [`rpc_ops!`][crate::rpc_ops]
//! macro. When using that macro, it's rarely necessary to use any of the
//! definitions in this module directly, however they are documented for the
//! sake of completeness.

use std::{
    marker::PhantomData,
    sync::{Arc, LazyLock},
};

use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use tokio::sync::SetOnce;

use crate::{
    config::{Binding, BindingType, ComponentConfig},
    runtime::{self, Component, Location},
};

/// A value that can be used as an RPC request or response.
///
/// Message types created by the [`rpc_ops!`][crate::rpc_ops] macro are
/// automatically given an `RpcMessage` impl.
pub trait RpcMessage: Serialize + for<'a> Deserialize<'a> + Send + 'static {
    fn verb(&self) -> &'static str;
}

/// A value that can be used as an RPC request handler.
///
/// The `Instance` struct created by the [`rpc_ops!`][crate::rpc_ops] macro is
/// automatically given an `Rpc` impl.
pub trait Rpc: Sync + Send + 'static {
    type Request: RpcMessage;
    type Response: RpcMessage;

    fn start() -> impl Future<Output = Self> + Send;

    fn handle(&self, q: Self::Request) -> impl Future<Output = RpcResult<Self::Response>> + Send;
}

type RpcInstance<R> = Arc<SetOnce<R>>;

/// An RPC component, parameterized by an `Rpc` impl.
///
/// This type is an implementation detail that you should not need to use
/// directly. The `ComponentConfig` returned by [`component`] is keyed by this
/// type.
pub struct RpcComponent<R>(PhantomData<R>);

impl<R: Rpc> Component for RpcComponent<R> {
    type Instance = RpcInstance<R>;
}

impl<R: Rpc> RpcComponent<R> {
    fn entry() -> BoxFuture<'static, ()> {
        Box::pin(async move {
            let instance = Arc::new(SetOnce::new());
            runtime::set_instance::<Self>(instance.clone());
            // we must call set_instance() asap, because get_instance::<T> blocks
            // until the corresponding set_instance::<T> is called and we do not
            // want to block in start() impls that make RPC calls.
            instance.set(R::start().await).ok().unwrap();
            rpc_http_server::<R>(instance).await;
            panic!("rpc_http_server exited");
        })
    }

    fn component(label: String) -> ComponentConfig {
        ComponentConfig {
            label,
            id: RpcComponent::<R>::id(),
            binding: BindingType::Http,
            is_stateful: false,
            entry: Self::entry,
        }
    }
}

/// Create a `ComponentConfig` for an `Rpc` impl.
///
/// When using the [`rpc_ops!`][crate::rpc_ops] macro, you should instead use
/// the `component` function defined by that macro.
pub fn component<R: Rpc>(label: String) -> ComponentConfig {
    RpcComponent::<R>::component(label)
}

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

/// A client for making requests to an RPC component.
///
/// Cloning values of this type will result in clients that share resources
/// such as connection pools.
///
/// The `Client` struct defined by the [`rpc_ops!`][crate::rpc_ops] macro is a
/// thin wrapper around this type.
pub struct RpcClient<R> {
    instance: Option<LazyLock<RpcInstance<R>>>,
}

impl<R: Rpc> Clone for RpcClient<R> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

fn init_client_instance<R: Rpc>() -> RpcInstance<R> {
    runtime::get_instance::<RpcComponent<R>>().clone()
}

impl<R: Rpc> RpcClient<R> {
    /// Create a new client for a particular `Rpc` impl. If an existing client
    /// can be cloned, that should be preferred, as it will result in resources
    /// being shared between the clients.
    pub fn new() -> RpcClient<R> {
        let instance = if runtime::is_local::<RpcComponent<R>>() {
            Some(LazyLock::new(
                init_client_instance::<R> as fn() -> RpcInstance<R>,
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
            Some(inner) => inner.wait().await.handle(q).await,
            None => http_call::<R>(q).await,
        }
    }

    /// Send a request to a specific location. This will always be sent over
    /// HTTP, even if the component is running in the same process.
    pub async fn call_at(&self, loc: Location, q: R::Request) -> RpcResult<R::Response> {
        http_call_at::<R>(loc, q).await
    }

    /// Returns a reference to the underlying `Rpc` impl if the target component
    /// is running in the same process.
    pub async fn local(&self) -> Option<&R> {
        match &self.instance {
            Some(inner) => Some(inner.wait().await),
            _ => None,
        }
    }
}

async fn rpc_http_server<R: Rpc>(inner: RpcInstance<R>) {
    let label = runtime::label::<RpcComponent<R>>();
    let addr = match runtime::binding::<RpcComponent<R>>() {
        Binding::Http(port) => ("0.0.0.0", port),
        _ => panic!("RPC component has non-HTTP binding"),
    };

    let path = format!("/{}/rpc", label);
    let app = axum::Router::new().route(
        &path,
        axum::routing::post(move |axum::Json(req): axum::Json<R::Request>| {
            let inner = inner.clone();
            async move {
                log::debug!("{} received RPC request: {}", label, req.verb());
                inner.wait().await.handle(req).await.map(|r| axum::Json(r))
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    log::info!("{} listening on {:?}", label, addr);
    axum::serve(listener, app).await.unwrap();
}

const HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    log::debug!("created global reqwest HTTP client");
    reqwest::Client::new()
});

async fn http_endpoint<R: Rpc>() -> RpcResult<String> {
    let label = runtime::label::<RpcComponent<R>>();
    match runtime::discover::<RpcComponent<R>>().await {
        Ok(loc) => match loc {
            Location::Http(endpoint) => Ok(endpoint),
        },
        Err(e) => Err(RpcError::Misc(format!(
            "could not discover endpoint for {}: {}",
            label, e
        ))),
    }
}

async fn http_call<R: Rpc>(q: R::Request) -> RpcResult<R::Response> {
    let loc = http_endpoint::<R>().await?;
    http_call_at::<R>(Location::Http(loc), q).await
}

async fn http_call_at<R: Rpc>(loc: Location, q: R::Request) -> RpcResult<R::Response> {
    let label = runtime::label::<RpcComponent<R>>();
    let url = match loc {
        Location::Http(endpoint) => format!("{}/{}/rpc", endpoint, label),
    };
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
