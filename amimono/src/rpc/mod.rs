use std::{marker::PhantomData, sync::Arc};

use futures::future::BoxFuture;

use crate::{Binding, BindingType, Component, ComponentMain, Label, Location, runtime};

mod macros;

pub trait RpcHandler: Send + Sync + Sized + 'static {
    type Request: serde::Serialize + for<'a> serde::Deserialize<'a> + Send + 'static;
    type Response: serde::Serialize + for<'a> serde::Deserialize<'a> + Send + 'static;

    fn handle(&self, q: Self::Request) -> impl Future<Output = Self::Response> + Send;
}

pub trait Rpc: Send + Sync + Sized + 'static {
    const LABEL: Label;

    type Handler: RpcHandler;

    fn start() -> impl Future<Output = Self::Handler> + Send;

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
    fn main_async(&'_ self) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            runtime::bind_local(R::start().await).await;
            RpcServer::<R::Handler>(runtime::connect_local(R::LABEL).await)
                .start_server()
                .await
        })
    }
}

struct RpcServer<R>(Arc<R>);

impl<R> Clone for RpcServer<R> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<R: RpcHandler> RpcServer<R> {
    async fn start_server(&self) {
        let label = runtime::current_label();

        let addr = match runtime::binding() {
            Binding::None => {
                log::warn!("{} not starting HTTP server: no binding", label);
                return;
            }
            Binding::Http(addr, _) => addr,
        };

        let app = axum::Router::new().route(
            "/rpc",
            axum::routing::post({
                let inner = self.0.clone();
                async move |body: String| {
                    log::debug!("{} <- {}", label, body);
                    let req: R::Request = serde_json::from_str(&body).unwrap();
                    let res: R::Response = inner.handle(req).await;
                    let res_body = serde_json::to_string(&res).unwrap();
                    log::debug!("{} -> {}", label, res_body);
                    res_body
                }
            }),
        );

        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        log::info!("{} listening on http://{}", label, addr);
        axum::serve(listener, app).await.unwrap();
    }
}

#[derive(Debug)]
pub enum RpcError {
    Misc,
}

pub enum RpcClient<R: Rpc> {
    Local(PhantomData<R>, Arc<R::Handler>),
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
    pub async fn new() -> RpcClient<R> {
        match runtime::locate(R::LABEL) {
            Location::Local => {
                RpcClient::Local(PhantomData, runtime::connect_local(R::LABEL).await)
            }
            Location::Remote(url) => {
                RpcClient::Remote(PhantomData, reqwest::Client::new(), format!("{}/rpc", url))
            }
            Location::Unreachable => panic!("{} not reachable", R::LABEL),
        }
    }

    pub async fn call(
        &self,
        q: <R::Handler as RpcHandler>::Request,
    ) -> Result<<R::Handler as RpcHandler>::Response, RpcError> {
        let res = match self {
            RpcClient::Local(_, local) => runtime::scope_label(R::LABEL, local.handle(q)).await,
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
