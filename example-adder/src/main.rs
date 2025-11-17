mod calc {
    use amimono::Component;

    mod ops {
        amimono::rpc_ops! {
            fn add(a: u64, b: u64) -> u64;
            fn mul(a: u64, b: u64) -> u64;
        }
    }

    pub struct CalcService;

    impl ops::Handler for CalcService {
        const LABEL: amimono::Label = "calc";

        async fn new() -> Self {
            CalcService
        }

        async fn add(&self, a: u64, b: u64) -> u64 {
            a + b
        }

        async fn mul(&self, a: u64, b: u64) -> u64 {
            a * b
        }
    }

    pub type CalcClient = ops::RpcClient<CalcService>;

    pub fn component() -> Component {
        ops::component::<CalcService>()
    }
}

mod adder {
    use amimono::Component;

    use crate::calc::CalcClient;

    mod ops {
        amimono::rpc_ops! {
            fn add(a: u64, b: u64) -> u64;
        }
    }

    pub struct Adder {
        calc: CalcClient,
    }

    impl ops::Handler for Adder {
        const LABEL: amimono::Label = "adder";

        async fn new() -> Self {
            Adder {
                calc: CalcClient::new().await,
            }
        }

        async fn add(&self, a: u64, b: u64) -> u64 {
            self.calc.add(a, b).await.unwrap()
        }
    }

    pub type AdderClient = ops::RpcClient<Adder>;

    pub fn component() -> Component {
        ops::component::<Adder>()
    }
}

mod doubler {
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    use amimono::{Component, Label};
    use tokio::sync::Mutex;

    use crate::calc::CalcClient;

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

    mod ops {
        amimono::rpc_ops! {
            fn double(x: u64) -> u64;
        }
    }

    pub struct Doubler {
        calc: CalcClient,
        time: Arc<Mutex<Timing>>,
    }

    impl ops::Handler for Doubler {
        const LABEL: Label = "doubler";

        async fn new() -> Doubler {
            Doubler {
                calc: CalcClient::new().await,
                time: Arc::new(Mutex::new(Timing::new())),
            }
        }

        async fn double(&self, a: u64) -> u64 {
            let start = Instant::now();
            let res = self.calc.mul(2, a).await.unwrap();
            let elapsed = start.elapsed();
            self.time.lock().await.report(elapsed);
            res
        }
    }

    pub type DoublerClient = ops::RpcClient<Doubler>;

    pub fn component() -> Component {
        ops::component::<Doubler>()
    }
}

mod driver {
    use std::time::Duration;

    use amimono::{BindingType, Component};
    use rand::Rng;

    use crate::{adder::AdderClient, doubler::DoublerClient};

    async fn driver_main() {
        let _adder = AdderClient::new().await;
        let doubler = DoublerClient::new().await;
        // TODO: this is an annoying thing I have to fix
        tokio::time::sleep(Duration::from_secs(1)).await;
        loop {
            let a = rand::rng().random_range(10..50);
            let _ = doubler.double(a).await.unwrap();
            tokio::time::sleep(Duration::from_secs_f32(0.5)).await;
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
                    .add_component(crate::calc::component())
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
