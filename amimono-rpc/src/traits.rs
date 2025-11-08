use amimono::{Configuration, Context};
use log::info;

use crate::component::RPCComponent;

pub trait RPC: Sized + 'static {
    const LABEL: &'static str;

    type Req: serde::Serialize + for<'a> serde::Deserialize<'a>;
    type Res: serde::Serialize + for<'a> serde::Deserialize<'a>;

    fn start<X: Context>(ctx: &X) -> Self;
    fn handle<X: Context>(&self, ctx: &X, req: Self::Req) -> Self::Res;

    fn place<X: Configuration>(cf: &mut X) {
        cf.place::<RPCComponent<Self>>();
    }
    fn call<X: Context>(ctx: &X, req: Self::Req) -> Result<Self::Res, ()> {
        let binding = ctx.locate::<RPCComponent<Self>>();
        info!("binding: {:?}", binding);
        Err(())
    }
}
