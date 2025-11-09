use amimono::{Configuration, Context};

use crate::component::RpcComponent;

pub trait Rpc: Send + Sync + Sized + 'static {
    const LABEL: &'static str;

    type Request: serde::Serialize + for<'a> serde::Deserialize<'a>;
    type Response: serde::Serialize + for<'a> serde::Deserialize<'a>;

    fn start<X: Context>(ctx: &X) -> impl Future<Output = Self>;
    fn handle<X: Context>(
        &self,
        ctx: &X,
        req: Self::Request,
    ) -> impl Future<Output = Self::Response> + Send;

    fn place<X: Configuration>(cf: &mut X) {
        cf.place::<RpcComponent<Self>>();
    }
}
