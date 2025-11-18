use std::thread;

use crate::runtime::ComponentRegistry;

pub mod config;
pub mod rpc;
pub mod runtime;

mod macros;

pub fn entry(cf: config::AppConfig) {
    let mut reg = ComponentRegistry::new();
    for job in cf.jobs() {
        for comp in job.components() {
            log::debug!("register {}", comp.label);
            (comp.register)(&mut reg, comp.label.clone());
        }
    }

    log::info!("initializing runtime");
    runtime::init(cf, reg);

    let mut threads = Vec::new();
    for job in runtime::config().jobs() {
        for comp in job.components() {
            log::debug!("spawn {}", comp.label);
            let th = thread::spawn(comp.entry);
            threads.push(th);
        }
    }

    log::info!("components started");
    for th in threads {
        th.join().unwrap();
    }
}
