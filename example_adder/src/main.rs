use std::{thread, time::Duration};

use amimono::{Application, Component, Configuration, Context};
use amimono_rpc::{Rpc, RpcClient, RpcClientBuilder};
use log::info;
use rand::Rng;

struct Adder;
impl Rpc for Adder {
    const LABEL: &'static str = "adder";
    type Request = (u64, u64);
    type Response = u64;
    async fn start<X: Context>(_ctx: &X) -> Self {
        info!("adder starting");
        Adder
    }
    async fn handle<X: Context>(&self, _ctx: &X, (a, b): (u64, u64)) -> u64 {
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
    async fn start<X: Context>(ctx: &X) -> Self {
        info!("doubler starting");
        Doubler {
            adder: RpcClientBuilder::new(ctx).get(),
        }
    }
    async fn handle<X: Context>(&self, _ctx: &X, a: u64) -> u64 {
        info!("doubler: doubling {} with adder", a);
        self.adder.call((a, a)).await
    }
}

struct Main;
impl Component for Main {
    const LABEL: &'static str = "main";
    async fn main<X: Context>(ctx: X) {
        let doubler: RpcClient<Doubler> = RpcClientBuilder::new(&ctx).get();
        loop {
            let a = rand::rng().random_range(10..50);
            info!("main: doubling {}", a);
            let b = doubler.call(a).await;
            info!("main: got {}", b);
            thread::sleep(Duration::from_secs(1));
        }
    }
}

struct ExampleAdder;
impl Application for ExampleAdder {
    const LABEL: &'static str = "example_adder";
    fn setup<X: Configuration>(&self, cf: &mut X) {
        Adder::place(cf);
        Doubler::place(cf);
        Main::place(cf);
    }
}

fn main() {
    env_logger::init();
    amimono::run(ExampleAdder);
}
