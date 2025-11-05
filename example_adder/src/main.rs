extern crate amimono;
extern crate env_logger;
extern crate log;
extern crate rand;

use std::time::Duration;

use amimono::{Application, Component, Configuration, Context, Cron, RPC};
use log::info;
use rand::Rng;

struct Adder;
impl Component for Adder {
    const LABEL: &'static str = "adder";
    fn init() -> Adder {
        info!("adder init");
        Adder
    }
}
impl RPC for Adder {
    type Request = (u64, u64);
    type Response = u64;
    fn handle<X: Context>(&self, _ctx: &X, (a, b): Self::Request) -> Self::Response {
        info!("adder: calculating {}+{}", a, b);
        a + b
    }
}

struct Doubler;
impl Component for Doubler {
    const LABEL: &'static str = "doubler";
    fn init() -> Doubler {
        info!("doubler init");
        Doubler
    }
}
impl RPC for Doubler {
    type Request = u64;
    type Response = u64;
    fn handle<X: Context>(&self, ctx: &X, a: Self::Request) -> Self::Response {
        info!("doubler: got {}", a);
        Adder::call(ctx, (a, a))
    }
}

#[test]
fn test_doubler() {
    use amimono::test::*;

    let ctx = {
        let mut ctx = TestContext::new();
        ctx.mock::<Adder>(|(a, b)| a + b);
        ctx.place(Doubler::init());
        ctx
    };

    assert_eq!(Doubler::call(&ctx, 5), 10);
    assert_eq!(Doubler::call(&ctx, 6), 12);
    assert_eq!(Doubler::call(&ctx, 7), 14);
}

struct Main;
impl Component for Main {
    const LABEL: &'static str = "main";
    fn init() -> Main {
        info!("main init");
        Main
    }
}
impl Cron for Main {
    const INTERVAL: Duration = Duration::from_secs(1);
    fn fire<X: Context>(&self, ctx: &X) {
        info!("main: calling doubler...");
        let x = Doubler::call(ctx, rand::rng().random_range(5..100));
        info!("main: got {}", x);
    }
}

struct ExampleAdder;
impl Application for ExampleAdder {
    fn setup<Cf: Configuration>(&self, cf: &mut Cf) {
        Adder::place(cf, 2);
        Doubler::place(cf, 1);
        Main::place(cf);
    }
}

fn main() {
    env_logger::init();
    amimono::local::run_local(ExampleAdder);
}
