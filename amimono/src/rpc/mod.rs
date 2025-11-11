use std::{marker::PhantomData, sync::Arc};

use futures::future::LocalBoxFuture;

use crate::{
    Component, ComponentMain, Label, Location, Runtime,
    runtime::{LocalBinding, LocalBindingHandler},
};

pub trait Rpc: Send + Sync + Sized + 'static {
    const LABEL: Label;

    type Request: 'static;
    type Response: 'static;

    fn start(rt: Runtime) -> impl Future<Output = Self>;
    fn handle(&self, rt: Runtime, q: Self::Request) -> impl Future<Output = Self::Response>;

    fn client(rt: Runtime) -> RpcClient<Self> {
        RpcClient::new(rt)
    }
    fn component() -> Component {
        let main: RpcComponentMain<Self> = RpcComponentMain::new();
        Component::new(Self::LABEL, main)
    }
}

struct RpcComponentMain<R>(PhantomData<R>);

impl<R> RpcComponentMain<R> {
    fn new() -> RpcComponentMain<R> {
        RpcComponentMain(PhantomData)
    }
}

impl<R: Rpc> ComponentMain for RpcComponentMain<R> {
    fn main_async(&self, rt: Runtime) -> LocalBoxFuture<()> {
        Box::pin(async move {
            let job = Arc::new(R::start(rt.clone()).await);
            let handler = RpcLocalBindingHandler(job.clone());
            rt.bind_local(LocalBinding::new(handler));
        })
    }
}

struct RpcLocalBindingHandler<R>(Arc<R>);

impl<R: Rpc> LocalBindingHandler<R::Request, R::Response> for RpcLocalBindingHandler<R> {
    fn call(&self, rt: Runtime, q: R::Request) -> LocalBoxFuture<R::Response> {
        Box::pin(self.0.handle(rt, q))
    }
}

pub enum RpcClient<R> {
    Local(PhantomData<R>),
}

impl<R: Rpc> RpcClient<R> {
    fn new(rt: Runtime) -> RpcClient<R> {
        match rt.locate(R::LABEL) {
            Location::Local => RpcClient::Local(PhantomData),
            _ => unimplemented!(),
        }
    }

    pub async fn call(&self, rt: Runtime, q: R::Request) -> Result<R::Response, ()> {
        match self {
            RpcClient::Local(_) => Ok(rt.call_local::<R::Request, R::Response>(R::LABEL, q).await),
        }
    }
}
