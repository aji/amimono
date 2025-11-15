mod adder {
    use amimono::{Component, Label, Rpc, Runtime};

    pub struct Adder;
    impl Rpc for Adder {
        const LABEL: Label = "adder";

        type Request = (u64, u64);
        type Response = u64;

        async fn start(_rt: &Runtime) -> Adder {
            Adder
        }
        async fn handle(&self, _rt: &Runtime, (a, b): &(u64, u64)) -> u64 {
            a + b
        }
    }
    pub fn component() -> Component {
        Adder::component()
    }
}

mod doubler {
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    use amimono::{Component, Label, Rpc, RpcClient, Runtime};
    use tokio::sync::Mutex;

    use crate::adder::Adder;

    struct Timing {
        skip: usize,
        time_ns: u128,
        count: u128,
    }

    impl Timing {
        fn new() -> Timing {
            Timing {
                skip: 5,
                time_ns: 0,
                count: 0,
            }
        }

        fn report(&mut self, elapsed: Duration) {
            if self.skip > 0 {
                self.skip -= 1;
                log::info!("skipping metrics for this request...");
                return;
            }
            self.time_ns += elapsed.as_nanos();
            self.count += 1;
            log::info!(
                "call took {:8}ns {:8}ns/req {:10}req/s",
                elapsed.as_nanos(),
                self.time_ns / self.count,
                (1_000_000_000.0 * self.count as f64 / self.time_ns as f64) as u64
            );
        }
    }

    pub struct Doubler {
        adder: RpcClient<Adder>,
        time: Arc<Mutex<Timing>>,
    }
    impl Rpc for Doubler {
        const LABEL: Label = "doubler";

        type Request = u64;
        type Response = u64;

        async fn start(rt: &Runtime) -> Doubler {
            Doubler {
                adder: Adder::client(rt).await,
                time: Arc::new(Mutex::new(Timing::new())),
            }
        }
        async fn handle(&self, rt: &Runtime, a: &u64) -> u64 {
            let start = Instant::now();
            let res = self.adder.call(rt, &(*a, *a)).await.unwrap();
            let elapsed = start.elapsed();
            self.time.lock().await.report(elapsed);
            res
        }
    }
    pub fn component() -> Component {
        Doubler::component()
    }
}

mod driver {
    use std::time::Duration;

    use amimono::{BindingType, Component, Rpc, Runtime};
    use rand::Rng;

    use crate::doubler::Doubler;

    async fn driver_main(rt: Runtime) {
        let doubler = Doubler::client(&rt).await;
        // TODO: this is an annoying thing I have to fix
        tokio::time::sleep(Duration::from_secs(1)).await;
        loop {
            let a = rand::rng().random_range(10..50);
            let _ = doubler.call(&rt, &a).await.unwrap();
            tokio::time::sleep(Duration::from_secs_f32(0.01)).await;
        }
    }

    pub fn component() -> Component {
        Component::from_async_fn("driver", BindingType::None, driver_main)
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
    amimono::entry(app::configure());
}
