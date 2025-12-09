//! Subsystem for building components with an RPC interface.
//!
//! It's recommended to use this module via the [`rpc_ops!`][crate::rpc_ops]
//! macro. When using that macro, it's rarely necessary to use any of the
//! definitions in this module directly, however they are documented for the
//! sake of completeness.

use std::{
    borrow::Borrow,
    fmt,
    net::SocketAddr,
    sync::{Arc, LazyLock},
    time::Duration,
};

use futures::{
    FutureExt,
    future::{BoxFuture, Shared},
};
use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};

use crate::{
    component::{Component, ComponentKind, Location},
    retry::{Retry, RetryError, RetryStrategy},
    runtime,
    util::StaticHashMap,
};

/// The port used for the RPC HTTP server
pub const PORT: u16 = 9099;

pub type RpcResult<T> = Result<T, RpcError>;

/// An error when making an RPC call.
#[derive(Clone, Serialize, Deserialize)]
pub enum RpcError {
    /// A spurious error with an unstructured string message. These can
    /// generally be assumed to be recoverable.
    Spurious(String),

    /// A miscellaneous error with an unstructured string message. These should
    /// generally be assumed to be unrecoverable.
    Misc(String),

    /// An error together with a location. This variant is constructed
    /// automatically by `RpcClient` when making a call, and can be nested
    /// several layers deep. Use `root_cause` to get the innermost `RpcError`.
    Downstream(String, Box<RpcError>),
}

impl RpcError {
    /// Unwrap layers of caused-by nesting to get the innermost error.
    pub fn root_cause(&self) -> &RpcError {
        match self {
            RpcError::Downstream(_, e) => e.root_cause(),
            _ => self,
        }
    }
}

impl RetryError for RpcError {
    fn should_retry(&self) -> bool {
        match self {
            RpcError::Spurious(_) => true,
            RpcError::Misc(_) => false,
            RpcError::Downstream(_, e) => e.should_retry(),
        }
    }
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

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcError::Spurious(s) => write!(f, "spurious: {s}"),
            RpcError::Misc(s) => write!(f, "rpc error: {s}"),
            RpcError::Downstream(at, e) => write!(f, "{at}: {e}"),
        }
    }
}

impl From<String> for RpcError {
    fn from(s: String) -> Self {
        RpcError::Misc(s)
    }
}

impl From<&str> for RpcError {
    fn from(value: &str) -> Self {
        RpcError::Misc(value.to_owned())
    }
}

impl From<crate::error::Error> for RpcError {
    fn from(value: crate::error::Error) -> Self {
        RpcError::Misc(format!("amimono error: {value}"))
    }
}

impl From<reqwest::Error> for RpcError {
    fn from(value: reqwest::Error) -> Self {
        if value.is_timeout() {
            let origin = match value.url() {
                Some(u) => u.origin().ascii_serialization(),
                None => "(unknown)".to_owned(),
            };
            RpcError::Spurious(format!("http timeout at {origin}"))
        } else {
            RpcError::Misc(format!("http error: {value}"))
        }
    }
}

impl From<serde_json::Error> for RpcError {
    fn from(value: serde_json::Error) -> Self {
        RpcError::Misc(format!("json error: {value}"))
    }
}

impl From<std::io::Error> for RpcError {
    fn from(value: std::io::Error) -> Self {
        RpcError::Misc(format!("io error: {value}"))
    }
}

impl From<tokio::task::JoinError> for RpcError {
    fn from(value: tokio::task::JoinError) -> Self {
        match value.try_into_panic() {
            Ok(e) => std::panic::resume_unwind(e),
            Err(e) => match e.is_cancelled() {
                true => RpcError::Misc(format!("task cancelled")),
                false => RpcError::Misc(format!("tokio join error")),
            },
        }
    }
}

/// A type that can be used as an RPC request or response.
///
/// Message types created by the [`rpc_ops!`][crate::rpc_ops] macro are
/// automatically given an `RpcMessage` impl.
pub trait RpcMessage: Serialize + for<'a> Deserialize<'a> + Send + Sync + 'static {
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
    fn handle<'i, 'q, 'f>(&'i self, q: &'q T::Request) -> BoxFuture<'f, RpcResult<T::Response>>
    where
        'i: 'f,
        'q: 'f;
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
        q: &<Self::Kind as RpcComponentKind>::Request,
    ) -> impl Future<Output = RpcResult<<Self::Kind as RpcComponentKind>::Response>> + Send;
}

impl<T: RpcComponent> RpcInstance<T::Kind> for T {
    fn handle<'i, 'q, 'f>(
        &'i self,
        q: &'q <T::Kind as RpcComponentKind>::Request,
    ) -> BoxFuture<'f, RpcResult<<T::Kind as RpcComponentKind>::Response>>
    where
        'i: 'f,
        'q: 'f,
    {
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
            set_instance(instance.clone()).await;
            let handler = Arc::new(DefaultHttpInstance::<T::Kind>(instance.clone()));
            HTTP_HANDLERS.insert(<Self::Kind as ComponentKind>::LABEL, handler);
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
pub struct RpcClient<T: RpcComponentKind, R = Retry> {
    retry: R,
    instance: Option<Shared<BoxFuture<'static, <T as ComponentKind>::Instance>>>,
}

/// The default retry strategy for RPC clients: 5 attempts with exponential
/// backoff, starting with a delay between 100 and 200 millis. If you have
/// specific requirements then feel free to use a different one, but this one
/// should be good for typical applications.
pub const DEFAULT_RETRY: Retry = Retry::delay_jitter_millis(100..=200)
    .with_max_attempts(5)
    .with_backoff();

impl<T: RpcComponentKind, R: Clone> Clone for RpcClient<T, R> {
    fn clone(&self) -> Self {
        RpcClient {
            retry: self.retry.clone(),
            instance: self.instance.clone(),
        }
    }
}

impl<T: RpcComponentKind, R: Sync> RpcClient<T, R> {
    pub fn with_retry<X>(self, retry: X) -> RpcClient<T, X> {
        RpcClient {
            retry,
            instance: self.instance,
        }
    }

    /// Send a request once. If the target `Rpc` impl belongs to a component
    /// that is running in the same process, this will result in the target
    /// handler being invoked directly.
    pub async fn call_once(&self, q: &T::Request) -> RpcResult<T::Response> {
        let res = match &self.instance {
            Some(inner) => inner.clone().await.handle(q).await,
            None => http_call::<T>(q).await,
        };
        res.map_err(|e| RpcError::Downstream(T::LABEL.to_owned(), Box::new(e)))
    }

    /// Send a request to a specific location. If the target location is the
    /// current location, this will be sent in-process. Otherwise, it will be sent
    /// over HTTP.
    pub async fn call_at_once<L, A>(&self, loc: L, q: &T::Request) -> RpcResult<T::Response>
    where
        L: Borrow<Location<A>>,
        A: Borrow<str>,
    {
        let addr = loc.borrow().addr();

        // TODO: not 100% sure why this box is needed but the futures types are
        // too complicated for rustc rpc_ops! handlers for some reason and I'm
        // choosing not to dig into it right now.
        let block: BoxFuture<'_, RpcResult<T::Response>> = Box::pin(async {
            if T::is_local()
                && T::myself().await.ok().as_ref().map(|x| x.addr()) == Some(addr)
                && let Some(inner) = &self.instance
            {
                inner.clone().await.handle(q).await
            } else {
                http_call_at::<T>(addr, q).await
            }
        });
        let res = block.await;
        res.map_err(|e| RpcError::Downstream(T::LABEL.to_owned(), Box::new(e)))
    }
}

impl<T: RpcComponentKind> RpcClient<T, Retry> {
    /// Create a new client for a particular `Rpc` impl. If an existing client
    /// can be cloned, that should be preferred, as it will result in resources
    /// being shared between the clients.
    pub fn new() -> RpcClient<T, Retry> {
        Self {
            retry: DEFAULT_RETRY.clone(),
            instance: T::instance().map(|x| x.boxed().shared()),
        }
    }
}

impl<T: RpcComponentKind, R: RetryStrategy<RpcError>> RpcClient<T, R> {
    /// Send a request, retrying the request according to the retry strategy.
    pub async fn call(&self, q: &T::Request) -> RpcResult<T::Response> {
        for num_attempts in 1.. {
            match self.call_once(q).await {
                Ok(x) => {
                    return Ok(x);
                }
                Err(e) => match self.retry.retry(num_attempts, &e) {
                    Some(delay) => {
                        log::warn!("retry after {delay:?}: {e}");
                        tokio::time::sleep(delay).await;
                    }
                    None => {
                        log::error!("no retries: {e}");
                        return Err(e);
                    }
                },
            }
        }
        unreachable!()
    }

    /// Send a request to a specific location, retrying the request according to
    /// the retry strategy.
    pub async fn call_at<L, A>(&self, loc: L, q: &T::Request) -> RpcResult<T::Response>
    where
        L: Borrow<Location<A>>,
        A: Borrow<str>,
    {
        let loc = loc.borrow();
        for num_attempts in 1.. {
            match self.call_at_once(loc, q).await {
                Ok(x) => {
                    return Ok(x);
                }
                Err(e) => match self.retry.retry(num_attempts, &e) {
                    Some(delay) => {
                        log::warn!("retry after {delay:?}: {e}");
                        tokio::time::sleep(delay).await;
                    }
                    None => {
                        log::error!("no retries: {e}");
                        return Err(e);
                    }
                },
            }
        }
        unreachable!()
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
            let a = self.0.handle(&q).await?;
            let res = match serde_json::to_vec(&a) {
                Ok(res) => res,
                Err(e) => Err(RpcError::Misc(format!("serialization failed: {e}")))?,
            };
            Ok(res)
        })
    }
}

static HTTP_HANDLERS: StaticHashMap<&'static str, dyn HttpInstance> = StaticHashMap::new();

static HTTP_SERVER: LazyLock<Shared<BoxFuture<'static, ()>>> = LazyLock::new(|| {
    let fut = rpc_http_server().boxed().shared();
    tokio::task::spawn(fut.clone());
    fut
});

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
                match HTTP_HANDLERS.get(label.as_str()) {
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

async fn http_call<R: RpcComponentKind>(q: &R::Request) -> RpcResult<R::Response> {
    let loc = match R::discover_running().await {
        Ok(locs) => match locs.choose(&mut rand::rng()) {
            Some(x) => x.clone(),
            None => return Err(RpcError::Misc(format!("discovery endpoints empty"))),
        },
        Err(e) => return Err(RpcError::Misc(format!("could not discover endpoint: {e}"))),
    };
    http_call_at::<R>(loc.addr(), q).await
}

async fn http_call_at<R: RpcComponentKind>(addr: &str, q: &R::Request) -> RpcResult<R::Response> {
    let label = R::LABEL;
    let url = format!("http://{}:{}/rpc/{}", addr, PORT, label);
    log::debug!("outgoing RPC: {} -> {}", label, url);
    let resp = HTTP_CLIENT
        .post(&url)
        .json(&q)
        .timeout(Duration::from_millis(rand::random_range(500..2000)))
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let msg = resp.json::<RpcError>().await?;
        return Err(msg);
    }
    let resp_msg = resp.json::<R::Response>().await?;
    Ok(resp_msg)
}
