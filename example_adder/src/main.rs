extern crate amimono;
extern crate amimono_rpc;
extern crate env_logger;
extern crate log;
extern crate rand;

use amimono::{Application, Configuration, Context};
use amimono_rpc::RPC;
use log::info;

struct Adder;
impl RPC for Adder {
    const LABEL: &'static str = "adder";
    type Req = (u64, u64);
    type Res = u64;
    fn start<X: Context>(_ctx: &X) -> Self {
        info!("adder starting");
        Adder
    }
    fn handle<X: Context>(&self, _ctx: &X, (a, b): Self::Req) -> Self::Res {
        a + b
    }
}

struct Doubler;
impl RPC for Doubler {
    const LABEL: &'static str = "doubler";
    type Req = u64;
    type Res = u64;
    fn start<X: Context>(ctx: &X) -> Self {
        info!("doubler starting");
        Adder::call(ctx, (3, 3)).unwrap();
        Doubler
    }
    fn handle<X: Context>(&self, ctx: &X, req: Self::Req) -> Self::Res {
        Adder::call(ctx, (req, req)).unwrap()
    }
}

struct ExampleAdder;
impl Application for ExampleAdder {
    const LABEL: &'static str = "example_adder";
    fn setup<X: Configuration>(&self, cf: &mut X) {
        Adder::place(cf);
        Doubler::place(cf);
    }
}

fn main() {
    env_logger::init();
    amimono::run(ExampleAdder);
}
