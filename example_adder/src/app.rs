use amimono::{AppBuilder, AppConfig, JobBuilder};

use crate::{adder, doubler, driver};

pub fn configure() -> AppConfig {
    AppBuilder::new()
        .add_job(
            JobBuilder::new()
                .add_component(adder::component())
                .add_component(doubler::component())
                .add_component(driver::component()),
        )
        .build()
}
