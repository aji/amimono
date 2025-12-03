mod calc {
    use amimono::rpc::RpcResult;

    mod ops {
        amimono::rpc_ops! {
            const LABEL: &'static str = "calc";

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

        async fn add(&self, a: u64, b: u64) -> RpcResult<u64> {
            Ok(a + b)
        }

        async fn mul(&self, a: u64, b: u64) -> RpcResult<u64> {
            Ok(a * b)
        }
    }

    pub type CalcClient = ops::Client<CalcService>;
    pub type CalcComponent = ops::ComponentImpl<CalcService>;
}

mod adder {
    use amimono::rpc::RpcResult;

    use crate::calc::CalcClient;

    mod ops {
        amimono::rpc_ops! {
            const LABEL: &'static str = "adder";

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

        async fn add(&self, a: u64, b: u64) -> RpcResult<u64> {
            self.calc.add(a, b).await
        }
    }

    pub type AdderClient = ops::Client<Adder>;
    pub type AdderComponent = ops::ComponentImpl<Adder>;
}

mod doubler {
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    use amimono::rpc::RpcResult;
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
            const LABEL: &'static str = "doubler";

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

        async fn double(&self, a: u64) -> RpcResult<u64> {
            let start = Instant::now();
            let res = self.calc.mul(2, a).await?;
            let elapsed = start.elapsed();
            self.time.lock().await.report(elapsed);
            Ok(res)
        }
    }

    pub type DoublerClient = ops::Client<Doubler>;
    pub type DoublerComponent = ops::ComponentImpl<Doubler>;
}

mod driver {
    use std::time::Duration;

    use amimono::component::{Component, ComponentImpl};
    use futures::future::BoxFuture;
    use rand::Rng;

    use crate::{adder::AdderClient, doubler::DoublerClient};

    pub struct Driver;

    impl Component for Driver {
        type Instance = ();
        const LABEL: &'static str = "driver";
    }

    impl ComponentImpl for Driver {
        type Component = Self;

        async fn main<F>(set_instance: F) -> ()
        where
            F: FnOnce(<Self::Component as Component>::Instance) -> BoxFuture<'static, ()> + Send,
        {
            set_instance(()).await;

            match Self::storage().await {
                Ok(path) => {
                    log::info!("storage path: {:?}", path);
                    if let Err(e) = std::fs::write(path.join("hello.txt"), "hello") {
                        log::warn!("failed to write to storage: {:?}", e);
                    }
                }
                Err(e) => {
                    log::warn!("failed to get storage path: {:?}", e);
                }
            }

            let _adder = AdderClient::new();
            let doubler = DoublerClient::new();
            loop {
                let a = rand::rng().random_range(10..50);
                match doubler.double(a).await {
                    Ok(_) => (),
                    Err(e) => log::error!("RPC error: {:?}", e),
                }
                tokio::time::sleep(Duration::from_secs_f32(10.0)).await;
            }
        }
    }
}

mod app {
    use amimono::{
        component::ComponentImpl,
        config::{AppBuilder, AppConfig, JobBuilder},
    };

    pub fn configure() -> AppConfig {
        AppBuilder::new(env!("APP_REVISION"))
            .add_job(
                JobBuilder::new()
                    .with_label("calc")
                    .install(crate::calc::CalcComponent::installer)
                    .install(crate::adder::AdderComponent::installer),
            )
            .add_job(
                JobBuilder::new()
                    .with_label("driver")
                    .install(crate::doubler::DoublerComponent::installer)
                    .install(crate::driver::Driver::installer),
            )
            .build()
    }
}

fn main() {
    env_logger::init();
    amimono::entry(app::configure());
}
