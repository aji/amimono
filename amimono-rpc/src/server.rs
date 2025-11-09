use std::{net::SocketAddr, sync::Arc};

use amimono::Runtime;
use axum::{Router, routing::post};
use log::info;
use tokio::net::TcpListener;

use crate::Rpc;

fn get_addr() -> SocketAddr {
    todo!()
}

pub async fn run_server<C: Rpc>(rt: Runtime, inner: C) -> () {
    let addr: SocketAddr = get_addr();

    let inner = Arc::new(inner);
    let app = Router::new().route(
        "/rpc",
        post({
            let rt = rt.clone();
            let inner = inner.clone();
            async move |body: String| {
                let req: C::Request = serde_json::from_str(&body).unwrap();
                let res: C::Response = inner.handle(rt, req).await;
                serde_json::to_string(&res).unwrap()
            }
        }),
    );

    let listener = TcpListener::bind(addr).await.unwrap();
    info!("{} listening on http://{}", C::LABEL, addr);
    axum::serve(listener, app).await.unwrap();
}
