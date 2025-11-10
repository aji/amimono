use amimono::{Component, Runtime};
use amimono_rpc::{Rpc, RpcClient};
use log::info;

use crate::adder::Adder;

pub struct Doubler {
    adder: RpcClient<Adder>,
}
impl Rpc for Doubler {
    const LABEL: &'static str = "doubler";
    type Request = u64;
    type Response = u64;
    async fn start(rt: &Runtime) -> Self {
        info!("doubler starting");
        Doubler {
            adder: Adder::client(rt),
        }
    }
    async fn handle(&self, _rt: &Runtime, a: u64) -> u64 {
        info!("doubler: doubling {} with adder", a);
        self.adder.call((a, a)).await
    }
}

pub fn component() -> Component {
    Doubler::component()
}
