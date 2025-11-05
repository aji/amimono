use std::time::Duration;

use amimono::{Application, Component, Configuration, Context, Cron, RPC};
use rand::Rng;

extern crate amimono;
extern crate rand;

struct Adder;
impl Component for Adder {
    const LABEL: &'static str = "adder";
    fn init() -> Adder {
        println!("adder init");
        Adder
    }
}
impl RPC for Adder {
    type Request = (u64, u64);
    type Response = u64;
    fn handle<X: Context>(&self, _ctx: &X, (a, b): Self::Request) -> Self::Response {
        println!("adder: calculating {}+{}", a, b);
        a + b
    }
}

struct Doubler;
impl Component for Doubler {
    const LABEL: &'static str = "doubler";
    fn init() -> Doubler {
        println!("doubler init");
        Doubler
    }
}
impl RPC for Doubler {
    type Request = u64;
    type Response = u64;
    fn handle<X: Context>(&self, ctx: &X, a: Self::Request) -> Self::Response {
        println!("doubler: got {}", a);
        Adder::call(ctx, (a, a))
    }
}

struct Main;
impl Component for Main {
    const LABEL: &'static str = "main";
    fn init() -> Main {
        println!("main init");
        Main
    }
}
impl Cron for Main {
    const INTERVAL: Duration = Duration::from_secs(1);
    fn fire<X: Context>(&self, ctx: &X) {
        println!("main: calling doubler...");
        let x = Doubler::call(ctx, rand::rng().random_range(5..100));
        println!("main: got {}", x);
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
    amimono::local::run_local(ExampleAdder);
}
