use amimono::Runtime;

use crate::component::RpcComponent;

pub trait Rpc: Send + Sync + Sized + 'static {
    const LABEL: &'static str;

    type Request: serde::Serialize + for<'a> serde::Deserialize<'a>;
    type Response: serde::Serialize + for<'a> serde::Deserialize<'a>;

    fn start(rt: Runtime) -> impl Future<Output = Self>;
    fn handle(
        &self,
        rt: Runtime,
        req: Self::Request,
    ) -> impl Future<Output = Self::Response> + Send;

    fn component() -> RpcComponent<Self> {
        RpcComponent::new()
    }
}
