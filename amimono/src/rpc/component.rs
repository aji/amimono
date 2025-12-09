use std::sync::Arc;

use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};

use crate::{
    component::{Component, ComponentKind},
    rpc::{RpcResult, http},
};

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
    const PORTS: &'static [u16] = &[http::PORT];
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
            let handler = Arc::new(http::DefaultHttpInstance::<T::Kind>(instance.clone()));
            http::HTTP_HANDLERS.insert(<Self::Kind as ComponentKind>::LABEL, handler);
            http::HTTP_SERVER.clone().await;
        })
    }
}
