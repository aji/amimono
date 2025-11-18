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

pub trait RpcMessage: Serialize + for<'a> Deserialize<'a> + Send + 'static {
    fn verb(&self) -> &'static str;
}

pub trait Rpc: Sync + Send + 'static {
    type Request: RpcMessage;
    type Response: RpcMessage;

    fn start() -> impl Future<Output = Self> + Send;

    fn handle(&self, q: Self::Request) -> impl Future<Output = Self::Response> + Send;
}

type RpcInstance<R> = Arc<SetOnce<R>>;

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

pub fn component<R: Rpc>(label: String) -> ComponentConfig {
    RpcComponent::<R>::component(label)
}

#[derive(Debug, Clone)]
pub enum RpcError {
    Misc(String),
}

pub enum RpcClient<R> {
    Local(LazyLock<RpcInstance<R>>),
}

impl<R: Rpc> Clone for RpcClient<R> {
    fn clone(&self) -> Self {
        RpcClient::<R>::new()
    }
}

impl<R: Rpc> RpcClient<R> {
    pub fn new() -> RpcClient<R> {
        RpcClient::Local(LazyLock::new(|| {
            runtime::get_instance::<RpcComponent<R>>().clone()
        }))
    }

    pub async fn call(&self, q: R::Request) -> Result<R::Response, RpcError> {
        match self {
            RpcClient::Local(instance) => Ok(instance.wait().await.handle(q).await),
        }
    }

    pub async fn local(&self) -> Option<&R> {
        match self {
            RpcClient::Local(instance) => Some(instance.wait().await),
        }
    }
}
