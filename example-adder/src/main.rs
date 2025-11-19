mod calc {
    use amimono::{config::ComponentConfig, rpc::RpcError};

    mod ops {
        amimono::rpc_ops! {
            fn add(a: u64, b: u64) -> u64;
            fn mul(a: u64, b: u64) -> u64;
        }
    }

    pub struct CalcService;

    impl ops::Handler for CalcService {
        async fn new() -> Self {
            log::info!("waiting...");
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            log::info!("done waiting.");
            CalcService
        }

        async fn add(&self, a: u64, b: u64) -> Result<u64, RpcError> {
            Ok(a + b)
        }

        async fn mul(&self, a: u64, b: u64) -> Result<u64, RpcError> {
            Ok(a * b)
        }
    }

    pub type CalcClient = ops::Client<CalcService>;

    pub fn component() -> ComponentConfig {
        ops::component::<CalcService>("calc".to_owned())
    }
}

mod adder {
    use amimono::{config::ComponentConfig, rpc::RpcError};

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
        async fn new() -> Self {
            Adder {
                calc: CalcClient::new(),
            }
        }

        async fn add(&self, a: u64, b: u64) -> Result<u64, RpcError> {
            self.calc.add(a, b).await
        }
    }

    pub type AdderClient = ops::Client<Adder>;

    pub fn component() -> ComponentConfig {
        ops::component::<Adder>("adder".to_owned())
    }
}

mod doubler {
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    use amimono::{config::ComponentConfig, rpc::RpcError};
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
                log::info!("call took {:8}ns", elapsed.as_nanos());
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
        async fn new() -> Doubler {
            Doubler {
                calc: CalcClient::new(),
                time: Arc::new(Mutex::new(Timing::new())),
            }
        }

        async fn double(&self, a: u64) -> Result<u64, RpcError> {
            let start = Instant::now();
            let res = self.calc.mul(2, a).await?;
            let elapsed = start.elapsed();
            self.time.lock().await.report(elapsed);
            Ok(res)
        }
    }

    pub type DoublerClient = ops::Client<Doubler>;

    pub fn component() -> ComponentConfig {
        ops::component::<Doubler>("doubler".to_owned())
    }
}

mod driver {
    use std::time::Duration;

    use amimono::{
        config::{BindingType, ComponentConfig},
        runtime::Component,
    };
    use rand::Rng;

    use crate::{adder::AdderClient, doubler::DoublerClient};

    struct Driver;
    impl Component for Driver {
        type Instance = ();
    }

    #[tokio::main]
    async fn driver_main() {
        let _adder = AdderClient::new();
        let doubler = DoublerClient::new();
        loop {
            let a = rand::rng().random_range(10..50);
            match doubler.double(a).await {
                Ok(_) => (),
                Err(e) => log::error!("RPC error: {:?}", e),
            }
            tokio::time::sleep(Duration::from_secs_f32(0.3)).await;
        }
    }

    pub fn component() -> ComponentConfig {
        ComponentConfig {
            label: "driver".to_owned(),
            id: Driver::id(),
            binding: BindingType::None,
            entry: driver_main,
        }
    }
}

mod app {
    use amimono::config::{AppBuilder, AppConfig, JobBuilder};

    pub fn configure() -> AppConfig {
        AppBuilder::new()
            .add_job(
                JobBuilder::new()
                    .with_label("calc")
                    .add_component(crate::calc::component())
                    .add_component(crate::adder::component()),
            )
            .add_job(
                JobBuilder::new()
                    .with_label("driver")
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
