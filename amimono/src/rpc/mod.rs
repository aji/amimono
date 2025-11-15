use std::{marker::PhantomData, sync::Arc};

use futures::{FutureExt, future::BoxFuture};

use crate::{Binding, BindingType, Component, ComponentMain, Label, Location, Runtime};

pub trait RpcHandler: Send + Sync + Sized + 'static {
    type Request: serde::Serialize + for<'a> serde::Deserialize<'a> + Send + 'static;
    type Response: serde::Serialize + for<'a> serde::Deserialize<'a> + Send + 'static;

    fn handle(&self, rt: &Runtime, q: Self::Request)
    -> impl Future<Output = Self::Response> + Send;
}

pub trait Rpc: Send + Sync + Sized + 'static {
    const LABEL: Label;

    type Handler: RpcHandler;
    type Client: From<RpcClient<Self>>;

    fn start(rt: &Runtime) -> impl Future<Output = Self::Handler> + Send;

    fn client(rt: &Runtime) -> impl Future<Output = Self::Client> {
        RpcClient::new(rt).map(Self::Client::from)
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
            let job = Arc::new(R::start(&rt).await);
            rt.bind_local(RpcLocal::<R>(job.clone()));
            let handler = RpcServer(job);
            handler.start_server(rt).await;
        })
    }
}

pub struct RpcLocal<R: Rpc>(Arc<R::Handler>);

struct RpcServer<R>(Arc<R>);

impl<R> Clone for RpcServer<R> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<R: RpcHandler> RpcServer<R> {
    async fn start_server(&self, rt: Runtime) {
        let addr = match rt.binding() {
            Binding::None => {
                log::warn!("{} not starting HTTP server: no binding", rt.label());
                return;
            }
            Binding::Http(addr, _) => addr,
        };

        let app = axum::Router::new().route(
            "/rpc",
            axum::routing::post({
                let rt = rt.clone();
                let inner = self.0.clone();
                async move |body: String| {
                    log::debug!("{} <- {}", rt.label(), body);
                    let req: R::Request = serde_json::from_str(&body).unwrap();
                    let res: R::Response = inner.handle(&rt, req).await;
                    let res_body = serde_json::to_string(&res).unwrap();
                    log::debug!("{} -> {}", rt.label(), res_body);
                    res_body
                }
            }),
        );

        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        log::info!("{} listening on http://{}", rt.label(), addr);
        axum::serve(listener, app).await.unwrap();
    }
}

pub enum RpcClient<R: Rpc> {
    Local(PhantomData<R>, Arc<RpcLocal<R>>),
    Remote(PhantomData<R>, reqwest::Client, String),
}

impl<R: Rpc> Clone for RpcClient<R> {
    fn clone(&self) -> Self {
        match self {
            Self::Local(_, local) => Self::Local(PhantomData, local.clone()),
            Self::Remote(_, client, url) => Self::Remote(PhantomData, client.clone(), url.clone()),
        }
    }
}

impl<R: Rpc> RpcClient<R> {
    async fn new(rt: &Runtime) -> RpcClient<R> {
        match rt.locate(R::LABEL) {
            Location::Local => RpcClient::Local(PhantomData, rt.connect_local(R::LABEL).await),
            Location::Remote(url) => {
                RpcClient::Remote(PhantomData, reqwest::Client::new(), format!("{}/rpc", url))
            }
            Location::Unreachable => panic!("{} not reachable", R::LABEL),
        }
    }
}

impl<R: Rpc> RpcHandler for RpcClient<R> {
    type Request = <<R as Rpc>::Handler as RpcHandler>::Request;
    type Response = Result<<<R as Rpc>::Handler as RpcHandler>::Response, ()>;

    async fn handle(&self, rt: &Runtime, q: Self::Request) -> Self::Response {
        let res = match self {
            RpcClient::Local(_, local) => local.0.handle(&rt.relocated(R::LABEL), q).await,
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
