use std::{marker::PhantomData, sync::Arc};

use futures::future::BoxFuture;
use log::info;

use crate::{Binding, BindingType, Component, ComponentMain, Label, Location, Runtime};

pub trait Rpc: Send + Sync + Sized + 'static {
    const LABEL: Label;

    type Request: serde::Serialize + for<'a> serde::Deserialize<'a> + Send + 'static;
    type Response: serde::Serialize + for<'a> serde::Deserialize<'a> + Send + 'static;

    fn start(rt: Runtime) -> impl Future<Output = Self> + Send;
    fn handle(&self, rt: Runtime, q: Self::Request) -> impl Future<Output = Self::Response> + Send;

    fn client(rt: Runtime) -> impl Future<Output = RpcClient<Self>> {
        RpcClient::new(rt)
    }
    fn component() -> Component {
        let main: RpcComponentMain<Self> = RpcComponentMain::new();
        Component::new(Self::LABEL, BindingType::Http, main)
    }
}

struct RpcComponentMain<R>(PhantomData<R>);

impl<R> RpcComponentMain<R> {
    fn new() -> RpcComponentMain<R> {
        RpcComponentMain(PhantomData)
    }
}

impl<R: Rpc> ComponentMain for RpcComponentMain<R> {
    fn main_async(&'_ self, rt: Runtime) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            let job = Arc::new(R::start(rt.clone()).await);
            rt.bind_local(RpcLocal(job.clone()));
            let handler = RpcServer(job);
            handler.start_server(rt).await;
        })
    }
}

pub struct RpcLocal<R>(Arc<R>);

struct RpcServer<R>(Arc<R>);

impl<R> Clone for RpcServer<R> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<R: Rpc> RpcServer<R> {
    async fn start_server(&self, rt: Runtime) {
        let addr = match rt.binding() {
            Binding::None => return,
            Binding::Http(addr, _) => addr,
        };

        let app = axum::Router::new().route(
            "/rpc",
            axum::routing::post({
                let rt = rt.clone();
                let inner = self.0.clone();
                async move |body: String| {
                    let req: R::Request = serde_json::from_str(&body).unwrap();
                    let res: R::Response = inner.handle(rt, req).await;
                    serde_json::to_string(&res).unwrap()
                }
            }),
        );

        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        info!("{} listening on http://{}", R::LABEL, addr);
        axum::serve(listener, app).await.unwrap();
    }
}

pub enum RpcClient<R> {
    Local(PhantomData<R>, Arc<RpcLocal<R>>),
    Remote(PhantomData<R>, reqwest::Client, String),
}

impl<R: Rpc> RpcClient<R> {
    async fn new(rt: Runtime) -> RpcClient<R> {
        match rt.locate(R::LABEL) {
            Location::Local => RpcClient::Local(PhantomData, rt.connect_local(R::LABEL).await),
            Location::Remote(url) => {
                RpcClient::Remote(PhantomData, reqwest::Client::new(), format!("{}/rpc", url))
            }
            Location::Unreachable => panic!("{} not reachable", R::LABEL),
        }
    }

    pub async fn call(&self, rt: Runtime, q: R::Request) -> Result<R::Response, ()> {
        let res = match self {
            RpcClient::Local(_, local) => local.0.handle(rt.relocated(R::LABEL), q).await,
            RpcClient::Remote(_, client, url) => client
                .post(url)
                .json(&q)
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap(),
        };
        Ok(res)
    }
}
