mod impls;
mod kinds;

use amimono::{
    component::Component,
    config::{AppBuilder, AppConfig, JobBuilder},
};

pub fn configure() -> AppConfig {
    AppBuilder::new(env!("APP_REVISION"))
        .add_job(
            JobBuilder::new()
                .with_label("calc")
                .install(crate::impls::calc::Component::installer)
                .install(crate::impls::adder::Component::installer),
        )
        .add_job(
            JobBuilder::new()
                .with_label("driver")
                .install(crate::impls::doubler::Component::installer)
                .install(crate::impls::driver::Driver::installer),
        )
        .build()
}

fn main() {
    env_logger::init();
    amimono::entry(configure());
}
