use std::marker::PhantomData;

use amimono::{Component, Label, Runtime};
use log::info;

use crate::{server::run_server, traits::Rpc};

pub struct RpcComponent<C>(PhantomData<C>);

impl<C: Rpc> RpcComponent<C> {
    pub fn new() -> RpcComponent<C> {
        RpcComponent(PhantomData)
    }
}

impl<C: Rpc> Component for RpcComponent<C> {
    fn label(&self) -> Label {
        C::LABEL
    }

    fn main(&self, rt: Runtime) {
        todo!()
    }
}
