use amimono::{Component, Runtime};
use amimono_rpc::{Rpc, rpc_component};
use log::info;

pub struct Adder;
impl Rpc for Adder {
    const LABEL: &'static str = "adder";
    type Request = (u64, u64);
    type Response = u64;
    async fn start(rt: &Runtime) -> Adder {
        info!("adder starting");
        Adder
    }
    async fn handle(&self, _rt: &Runtime, (a, b): (u64, u64)) -> u64 {
        info!("adder: calculating {} + {}", a, b);
        a + b
    }
}

pub fn component() -> Component {
    Adder::component()
}
