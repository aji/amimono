mod calc {
    use amimono::config::ComponentConfig;

    mod ops {
        amimono::rpc_ops! {
            fn add(a: u64, b: u64) -> u64;
            fn mul(a: u64, b: u64) -> u64;
        }
    }

    pub struct Calc;

    impl ops::Handler for Calc {
        fn new() -> Calc {
            Calc
        }

        fn add(&self, a: u64, b: u64) -> u64 {
            a + b
        }

        fn mul(&self, a: u64, b: u64) -> u64 {
            a * b
        }
    }

    pub type CalcClient = ops::Client<Calc>;

    pub fn component() -> ComponentConfig {
        ops::component::<Calc>("calc".to_string())
    }
}

mod driver {
    use amimono::config::{BindingType, ComponentConfig};

    use crate::calc::CalcClient;

    fn driver_entry() {
        let client = CalcClient::new();
        println!("3 + 5 = {}", client.add(3, 5).unwrap());
    }

    pub fn component() -> ComponentConfig {
        ComponentConfig {
            label: "driver".to_owned(),
            binding: BindingType::None,
            register: |_| {},
            entry: driver_entry,
        }
    }
}

mod app {
    use amimono::config::{AppBuilder, AppConfig};

    use crate::{calc, driver};

    pub fn configure() -> AppConfig {
        AppBuilder::new()
            .add_job(calc::component())
            .add_job(driver::component())
            .build()
    }
}

pub fn main() {
    env_logger::init();
    amimono::entry(app::configure());
}
