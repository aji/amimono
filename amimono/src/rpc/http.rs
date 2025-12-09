use std::{net::SocketAddr, sync::LazyLock, time::Duration};

use futures::{
    FutureExt,
    future::{BoxFuture, Shared},
};
use rand::seq::IndexedRandom;

use crate::{
    component::ComponentKind,
    rpc::{RpcComponentKind, RpcError, RpcResult},
    util::StaticHashMap,
};

/// The port used for the RPC HTTP server
pub const PORT: u16 = 9099;

pub trait HttpInstance: Send + Sync + 'static {
    fn handle_json<'h, 'q, 'f>(&'h self, q: &'q [u8]) -> BoxFuture<'f, RpcResult<Vec<u8>>>
    where
        'h: 'f,
        'q: 'f;
}

pub struct DefaultHttpInstance<T: RpcComponentKind>(pub <T as ComponentKind>::Instance);

impl<T: RpcComponentKind> HttpInstance for DefaultHttpInstance<T> {
    fn handle_json<'h, 'q, 'f>(&'h self, q: &'q [u8]) -> BoxFuture<'f, RpcResult<Vec<u8>>>
    where
        'h: 'f,
        'q: 'f,
    {
        Box::pin(async {
            let q = match serde_json::from_slice::<T::Request>(q) {
                Ok(q) => q,
                Err(e) => Err(RpcError::Misc(format!("request parse error: {e}")))?,
            };
            let a = self.0.handle(&q).await?;
            let res = match serde_json::to_vec(&a) {
                Ok(res) => res,
                Err(e) => Err(RpcError::Misc(format!("serialization failed: {e}")))?,
            };
            Ok(res)
        })
    }
}

pub static HTTP_HANDLERS: StaticHashMap<&'static str, dyn HttpInstance> = StaticHashMap::new();

pub static HTTP_SERVER: LazyLock<Shared<BoxFuture<'static, ()>>> = LazyLock::new(|| {
    let fut = rpc_http_server().boxed().shared();
    tokio::task::spawn(fut.clone());
    fut
});

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    log::debug!("created global reqwest HTTP client");
    reqwest::Client::new()
});

async fn rpc_http_server() {
    let app = axum::Router::new().route(
        "/rpc/{label}",
        axum::routing::post(
            async |axum::extract::Path(label): axum::extract::Path<String>,
                   body: axum::body::Bytes| {
                let bytes = body.to_vec();
                match HTTP_HANDLERS.get(label.as_str()) {
                    Some(h) => h.handle_json(&bytes).await,
                    None => Err(RpcError::Misc(format!("no handler for {label}"))),
                }
            },
        ),
    );

    let addr: SocketAddr = crate::runtime::to_addr(PORT);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    log::info!("rpc server listening on {:?}", addr);
    axum::serve(listener, app).await.unwrap();
}

pub async fn http_call<R: RpcComponentKind>(q: &R::Request) -> RpcResult<R::Response> {
    let loc = match R::discover_running().await {
        Ok(locs) => match locs.choose(&mut rand::rng()) {
            Some(x) => x.clone(),
            None => return Err(RpcError::Misc(format!("discovery endpoints empty"))),
        },
        Err(e) => return Err(RpcError::Misc(format!("could not discover endpoint: {e}"))),
    };
    http_call_at::<R>(loc.addr(), q).await
}

pub async fn http_call_at<R: RpcComponentKind>(
    addr: &str,
    q: &R::Request,
) -> RpcResult<R::Response> {
    let label = R::LABEL;
    let url = format!("http://{}:{}/rpc/{}", addr, PORT, label);
    log::debug!("outgoing RPC: {} -> {}", label, url);
    let resp = HTTP_CLIENT
        .post(&url)
        .json(&q)
        .timeout(Duration::from_millis(rand::random_range(500..2000)))
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let msg = resp.json::<RpcError>().await?;
        return Err(msg);
    }
    let resp_msg = resp.json::<R::Response>().await?;
    Ok(resp_msg)
}
