use std::marker::PhantomData;

use amimono::{BindingType, Component, Context};
use log::info;

use crate::{server::run_server, traits::Rpc};

pub struct RpcComponent<C>(PhantomData<C>);

impl<C: Rpc> Component for RpcComponent<C> {
    const LABEL: &'static str = C::LABEL;
    const BINDING: BindingType = BindingType::TCP(1);

    async fn main<X: Context>(ctx: X) {
        info!("start {} on {:?}", C::LABEL, ctx.binding());
        let inner = C::start(&ctx).await;
        run_server(ctx, inner).await;
    }
}
