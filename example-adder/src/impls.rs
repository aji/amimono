pub mod calc {
    use std::time::Duration;

    use amimono::rpc::RpcResult;

    pub struct CalcService;

    impl crate::kinds::calc::Handler for CalcService {
        async fn new() -> Self {
            tokio::time::sleep(Duration::from_millis(2)).await;
            CalcService
        }

        async fn add(&self, a: u64, b: u64) -> RpcResult<u64> {
            Ok(a + b)
        }

        async fn mul(&self, a: u64, b: u64) -> RpcResult<u64> {
            Ok(a * b)
        }
    }

    pub type Component = crate::kinds::calc::Component<CalcService>;
}

pub mod adder {
    use amimono::rpc::RpcResult;

    pub struct AdderService {
        calc: crate::kinds::calc::Client,
    }

    impl crate::kinds::adder::Handler for AdderService {
        async fn new() -> Self {
            AdderService {
                calc: Default::default(),
            }
        }

        async fn add(&self, a: u64, b: u64) -> RpcResult<u64> {
            self.calc.add(a, b).await
        }
    }

    pub type Component = crate::kinds::adder::Component<AdderService>;
}

pub mod doubler {
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    use amimono::rpc::RpcResult;
    use tokio::sync::Mutex;

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

    pub struct DoublerService {
        calc: crate::kinds::calc::Client,
        time: Arc<Mutex<Timing>>,
    }

    impl crate::kinds::doubler::Handler for DoublerService {
        async fn new() -> Self {
            DoublerService {
                calc: Default::default(),
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

    pub type Component = crate::kinds::doubler::Component<DoublerService>;
}

pub mod driver {
    use std::time::Duration;

    use amimono::component::Component;
    use futures::future::BoxFuture;
    use rand::Rng;

    pub struct Driver;

    impl Component for Driver {
        type Kind = crate::kinds::driver::DriverKind;

        async fn main<F>(set_instance: F)
        where
            F: FnOnce(()) -> BoxFuture<'static, ()> + Send,
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

            let _adder = crate::kinds::adder::Client::new();
            let doubler = crate::kinds::doubler::Client::new();
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
