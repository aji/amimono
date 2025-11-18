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

use serde::{Deserialize, Serialize};
use tokio::sync::SetOnce;

use crate::{
    config::{BindingType, ComponentConfig},
    runtime::{self, Component},
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

    fn handle(&self, q: Self::Request) -> impl Future<Output = Self::Response> + Send;
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
    #[tokio::main]
    async fn entry() {
        let instance = Arc::new(SetOnce::new());
        runtime::set_instance::<Self>(instance.clone());
        // we must call set_instance() asap, because get_instance::<T> blocks
        // until the corresponding set_instance::<T> is called and we do not
        // want to block in start() impls that make RPC calls.
        instance.set(R::start().await).ok().unwrap();
    }

    fn component(label: String) -> ComponentConfig {
        ComponentConfig {
            label,
            id: RpcComponent::<R>::id(),
            binding: BindingType::Http,
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

/// An error when making an RPC call.
#[derive(Debug, Clone)]
pub enum RpcError {
    /// A miscellaneous error with an unstructured string message. These should
    /// generally be assumed to be unrecoverable.
    Misc(String),
}

/// A client for making requests to an RPC component.
///
/// Cloning values of this type will result in clients that share resources
/// such as connection pools.
///
/// The `Client` struct defined by the [`rpc_ops!`][crate::rpc_ops] macro is a
/// thin wrapper around this type.
pub enum RpcClient<R> {
    Local(LazyLock<RpcInstance<R>>),
}

impl<R: Rpc> Clone for RpcClient<R> {
    fn clone(&self) -> Self {
        RpcClient::<R>::new()
    }
}

impl<R: Rpc> RpcClient<R> {
    /// Create a new client for a particular `Rpc` impl. If an existing client
    /// can be cloned, that should be preferred, as it will result in resources
    /// being shared between the clients.
    pub fn new() -> RpcClient<R> {
        RpcClient::Local(LazyLock::new(|| {
            runtime::get_instance::<RpcComponent<R>>().clone()
        }))
    }

    /// Send a request. If the target `Rpc` impl belongs to a component that is
    /// running in the same process, this will result in the target handler
    /// being invoked directly.
    pub async fn call(&self, q: R::Request) -> Result<R::Response, RpcError> {
        match self {
            RpcClient::Local(instance) => Ok(instance.wait().await.handle(q).await),
        }
    }

    /// Returns a reference to the underlying `Rpc` impl if the target component
    /// is running in the same process.
    pub async fn local(&self) -> Option<&R> {
        match self {
            RpcClient::Local(instance) => Some(instance.wait().await),
        }
    }
}
