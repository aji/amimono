use std::{net::SocketAddr, sync::Arc};

use amimono::Context;
use axum::{Router, routing::post};
use log::info;
use tokio::net::TcpListener;

use crate::Rpc;

pub async fn run_server<X: Context, C: Rpc>(ctx: X, inner: C) -> () {
    let addr: SocketAddr = match ctx.binding() {
        amimono::LocalBinding::None => panic!(),
        amimono::LocalBinding::TCP(addrs) => addrs[0],
    };

    let ctx = Arc::new(ctx);
    let inner = Arc::new(inner);
    let app = Router::new().route(
        "/rpc",
        post({
            let ctx = ctx.clone();
            let inner = inner.clone();
            async move |body: String| {
                let req: C::Request = serde_json::from_str(&body).unwrap();
                let res: C::Response = inner.handle(&*ctx, req).await;
                serde_json::to_string(&res).unwrap()
            }
        }),
    );

    let listener = TcpListener::bind(addr).await.unwrap();
    info!("{} listening on http://{}", C::LABEL, addr);
    axum::serve(listener, app).await.unwrap();
}
