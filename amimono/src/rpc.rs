use std::{
    marker::PhantomData,
    sync::{Arc, LazyLock},
};

use serde::{Deserialize, Serialize};
use tokio::sync::SetOnce;

use crate::{
    config::{BindingType, ComponentConfig},
    runtime::{self, Component, ComponentRegistry},
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
    fn register(reg: &mut ComponentRegistry, label: String) {
        reg.register::<Self>(label, Arc::new(SetOnce::new()))
    }

    #[tokio::main]
    async fn entry() {
        let instance = runtime::instance::<Self>().unwrap().clone();
        instance
            .set(R::start().await)
            .ok()
            .expect("could not set instance");
        // TODO
    }

    fn component(label: String) -> ComponentConfig {
        ComponentConfig {
            label,
            binding: BindingType::Http,
            register: Self::register,
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
            runtime::instance::<RpcComponent<R>>()
                .expect("no local instance")
                .clone()
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
