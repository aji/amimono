use std::marker::PhantomData;

use amimono::{BindingType, Component, Context};
use log::info;

use crate::traits::RPC;

pub struct RPCComponent<C>(PhantomData<C>);

impl<C: RPC> Component for RPCComponent<C> {
    const LABEL: &'static str = C::LABEL;
    const BINDING: BindingType = BindingType::TCP(1);

    fn main<X: Context>(ctx: &X) {
        info!("start {} on {:?}", C::LABEL, ctx.binding());
        let job = C::start(ctx);
    }
}
