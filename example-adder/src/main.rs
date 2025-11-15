mod calc {
    use amimono::{Component, Label, Rpc, RpcClient, RpcHandler, Runtime};
    use serde::{Deserialize, Serialize};

    pub trait Calc: Send + Sync + Sized + 'static {
        fn add(&self, rt: &Runtime, a: u64, b: u64) -> impl Future<Output = u64> + Send;
        fn mul(&self, rt: &Runtime, a: u64, b: u64) -> impl Future<Output = u64> + Send;
    }

    pub struct CalcService;

    pub type CalcClient = HandlerToCalc<RpcClient<CalcService>>;

    impl Calc for CalcService {
        async fn add(&self, _rt: &Runtime, a: u64, b: u64) -> u64 {
            a + b
        }
        async fn mul(&self, _rt: &Runtime, a: u64, b: u64) -> u64 {
            a * b
        }
    }

    impl Rpc for CalcService {
        const LABEL: Label = "calc";

        type Handler = CalcToHandler<CalcService>;
        type Client = CalcClient;

        async fn start(_rt: &Runtime) -> Self::Handler {
            CalcService.into()
        }
    }

    pub async fn client(rt: &Runtime) -> CalcClient {
        CalcService::client(rt).await
    }
    pub fn component() -> Component {
        CalcService::component()
    }

    #[derive(Serialize, Deserialize)]
    pub enum CalcRequest {
        Add(u64, u64),
        Mul(u64, u64),
    }

    #[derive(Serialize, Deserialize)]
    pub enum CalcResponse {
        Add(u64),
        Mul(u64),
    }

    pub struct CalcToHandler<T>(T);

    impl<T: Calc> From<T> for CalcToHandler<T> {
        fn from(value: T) -> Self {
            CalcToHandler(value)
        }
    }

    impl<T: Calc> RpcHandler for CalcToHandler<T> {
        type Request = CalcRequest;
        type Response = CalcResponse;

        async fn handle(&self, rt: &Runtime, q: Self::Request) -> Self::Response {
            match q {
                CalcRequest::Add(a, b) => CalcResponse::Add(self.0.add(rt, a, b).await),
                CalcRequest::Mul(a, b) => CalcResponse::Mul(self.0.mul(rt, a, b).await),
            }
        }
    }

    pub struct HandlerToCalc<T>(T);

    impl<T: RpcHandler<Request = CalcRequest, Response = Result<CalcResponse, ()>>> Calc
        for HandlerToCalc<T>
    {
        async fn add(&self, rt: &Runtime, a: u64, b: u64) -> u64 {
            match self.0.handle(rt, CalcRequest::Add(a, b)).await {
                Ok(CalcResponse::Add(a)) => a,
                _ => panic!(),
            }
        }
        async fn mul(&self, rt: &Runtime, a: u64, b: u64) -> u64 {
            match self.0.handle(rt, CalcRequest::Mul(a, b)).await {
                Ok(CalcResponse::Mul(a)) => a,
                _ => panic!(),
            }
        }
    }

    impl<T> From<T> for HandlerToCalc<T> {
        fn from(value: T) -> Self {
            HandlerToCalc(value)
        }
    }
}

mod adder {
    use amimono::{Component, Label, Rpc, RpcClient, RpcHandler, Runtime};
    use rand::Rng;

    use crate::calc::{Calc, CalcClient};

    pub struct Adder {
        calc: CalcClient,
    }

    impl RpcHandler for Adder {
        type Request = (u64, u64);
        type Response = u64;

        async fn handle(&self, rt: &Runtime, (a, b): (u64, u64)) -> u64 {
            if rand::rng().random_bool(0.5) {
                a + b
            } else {
                self.calc.add(rt, a, b).await
            }
        }
    }

    impl Rpc for Adder {
        const LABEL: Label = "adder";

        type Handler = Self;
        type Client = RpcClient<Self>;

        async fn start(rt: &Runtime) -> Adder {
            Adder {
                calc: crate::calc::client(rt).await,
            }
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

    use amimono::{Component, Label, Rpc, RpcClient, RpcHandler, Runtime};
    use rand::Rng;
    use tokio::sync::Mutex;

    use crate::{
        adder::Adder,
        calc::{Calc, CalcClient},
    };

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
        calc: CalcClient,
        adder: RpcClient<Adder>,
        time: Arc<Mutex<Timing>>,
    }

    impl RpcHandler for Doubler {
        type Request = u64;
        type Response = u64;

        async fn handle(&self, rt: &Runtime, a: u64) -> u64 {
            let start = Instant::now();
            let res = if rand::rng().random_bool(0.0) {
                self.adder.handle(rt, (a, a)).await.unwrap()
            } else {
                self.calc.mul(rt, 2, a).await
            };
            let elapsed = start.elapsed();
            self.time.lock().await.report(elapsed);
            res
        }
    }

    impl Rpc for Doubler {
        const LABEL: Label = "doubler";

        type Handler = Self;
        type Client = RpcClient<Self>;

        async fn start(rt: &Runtime) -> Doubler {
            Doubler {
                calc: crate::calc::client(rt).await,
                adder: Adder::client(rt).await,
                time: Arc::new(Mutex::new(Timing::new())),
            }
        }
    }

    pub fn component() -> Component {
        Doubler::component()
    }
}

mod driver {
    use std::time::Duration;

    use amimono::{BindingType, Component, Rpc, RpcHandler, Runtime};
    use rand::Rng;

    use crate::doubler::Doubler;

    async fn driver_main(rt: Runtime) {
        let doubler = Doubler::client(&rt).await;
        // TODO: this is an annoying thing I have to fix
        tokio::time::sleep(Duration::from_secs(1)).await;
        loop {
            let a = rand::rng().random_range(10..50);
            let _ = doubler.handle(&rt, a).await.unwrap();
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
