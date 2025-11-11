mod adder {
    use std::time::Duration;

    use amimono::{Component, Label, Rpc, Runtime};
    use log::info;

    pub struct Adder;
    impl Rpc for Adder {
        const LABEL: Label = "adder";

        type Request = (u64, u64);
        type Response = u64;

        async fn start(_rt: Runtime) -> Adder {
            info!("starting...");
            tokio::time::sleep(Duration::from_secs(3)).await;
            info!("started");
            Adder
        }
        async fn handle(&self, _rt: Runtime, (a, b): (u64, u64)) -> u64 {
            info!("calculating {} + {}", a, b);
            a + b
        }
    }
    pub fn component() -> Component {
        Adder::component()
    }
}

mod doubler {
    use std::time::Duration;

    use amimono::{Component, Label, Rpc, RpcClient, Runtime};
    use log::info;

    use crate::adder::Adder;

    pub struct Doubler {
        adder: RpcClient<Adder>,
    }
    impl Rpc for Doubler {
        const LABEL: Label = "doubler";

        type Request = u64;
        type Response = u64;

        async fn start(rt: Runtime) -> Doubler {
            info!("starting...");
            tokio::time::sleep(Duration::from_secs(1)).await;
            info!("started");
            Doubler {
                adder: Adder::client(rt),
            }
        }
        async fn handle(&self, rt: Runtime, a: u64) -> u64 {
            info!("doubling {} via adder", a);
            self.adder.call(rt, (a, a)).await.unwrap()
        }
    }
    pub fn component() -> Component {
        Doubler::component()
    }
}

mod driver {
    use std::time::Duration;

    use amimono::{Component, Rpc, Runtime};
    use log::info;
    use rand::Rng;

    use crate::doubler::Doubler;

    async fn driver_main(rt: Runtime) {
        let doubler = Doubler::client(rt.clone());
        loop {
            let a = rand::rng().random_range(10..50);
            info!("doubling {} via doubler", a);
            let b = doubler.call(rt.clone(), a).await.unwrap();
            info!("got {}", b);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    pub fn component() -> Component {
        Component::from_async_fn("driver", driver_main)
    }
}

mod app {
    use amimono::{AppBuilder, AppConfig, JobBuilder};

    pub fn configure() -> AppConfig {
        AppBuilder::new()
            .add_job(
                JobBuilder::new()
                    .with_label("example")
                    .add_component(crate::adder::component())
                    .add_component(crate::doubler::component())
                    .add_component(crate::driver::component()),
            )
            .build()
    }
}

fn main() {
    env_logger::init();
    amimono::run_local(app::configure());
}
