use std::{thread, time::Duration};

use amimono::{AppBuilder, AppConfig, Component, JobBuilder, Runtime};
use amimono_rpc::{Rpc, RpcClient, RpcClientBuilder};
use log::info;
use rand::Rng;

struct Adder;
impl Rpc for Adder {
    const LABEL: &'static str = "adder";
    type Request = (u64, u64);
    type Response = u64;
    async fn start(rt: Runtime) -> Adder {
        info!("adder starting");
        Adder
    }
    async fn handle(&self, _rt: Runtime, (a, b): (u64, u64)) -> u64 {
        info!("adder: calculating {} + {}", a, b);
        a + b
    }
}

struct Doubler {
    adder: RpcClient<Adder>,
}
impl Rpc for Doubler {
    const LABEL: &'static str = "doubler";
    type Request = u64;
    type Response = u64;
    async fn start(rt: Runtime) -> Self {
        info!("doubler starting");
        Doubler {
            adder: RpcClientBuilder::new(rt).get(),
        }
    }
    async fn handle(&self, _rt: Runtime, a: u64) -> u64 {
        info!("doubler: doubling {} with adder", a);
        self.adder.call((a, a)).await
    }
}

struct Main;
impl Component for Main {
    fn label(&self) -> amimono::Label {
        "main"
    }
    fn main(&self, rt: Runtime) {
        let job = async {
            let doubler: RpcClient<Doubler> = RpcClientBuilder::new(rt).get();
            loop {
                let a = rand::rng().random_range(10..50);
                info!("main: doubling {}", a);
                let b = doubler.call(a).await;
                info!("main: got {}", b);
                thread::sleep(Duration::from_secs(1));
            }
        };
    }
}

fn configure() -> AppConfig {
    AppBuilder::new()
        .add_job(
            JobBuilder::new()
                .add_component(Adder::component())
                .add_component(Doubler::component())
                .add_component(Main),
        )
        .build()
}

fn main() {
    env_logger::init();
    amimono::run(configure());
}
