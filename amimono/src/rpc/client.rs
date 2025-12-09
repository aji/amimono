use std::borrow::Borrow;

use futures::{
    FutureExt,
    future::{BoxFuture, Shared},
};

use crate::{
    component::{ComponentKind, Location},
    retry::{Retry, RetryStrategy},
    rpc::{RpcComponentKind, RpcError, RpcResult, http},
};

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
            None => http::http_call::<T>(q).await,
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
                http::http_call_at::<T>(addr, q).await
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
