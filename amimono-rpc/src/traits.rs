use amimono::{Component, Runtime};

use crate::{RpcClient, RpcClientBuilder, rpc_component};

pub trait Rpc: Send + Sync + Sized + 'static {
    const LABEL: &'static str;

    type Request: serde::Serialize + for<'a> serde::Deserialize<'a> + Send + Sync + 'static;
    type Response: serde::Serialize + for<'a> serde::Deserialize<'a> + Send + Sync + 'static;

    fn start(rt: &Runtime) -> impl Future<Output = Self> + Send;
    fn handle(
        &self,
        rt: &Runtime,
        req: Self::Request,
    ) -> impl Future<Output = Self::Response> + Send;

    fn client(rt: &Runtime) -> RpcClient<Self> {
        RpcClientBuilder::new(rt).get()
    }
    fn component() -> Component {
        rpc_component::<Self>(Self::LABEL)
    }
}
