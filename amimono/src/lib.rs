use std::thread;

use crate::component::ComponentRegistry;

pub mod component;
pub mod config;
pub mod rpc;
pub mod runtime;

pub fn entry(cf: config::AppConfig) {
    runtime::init(cf);

    let _ = {
        let mut reg = ComponentRegistry::new();
        for job in runtime::config().jobs() {
            for comp in job.components() {
                (comp.register)(&mut reg);
            }
        }
        component::set_registry(reg);
    };

    let mut threads = Vec::new();
    for job in runtime::config().jobs() {
        for comp in job.components() {
            let th = thread::spawn(comp.entry);
            threads.push(th);
        }
    }
    for th in threads {
        th.join().unwrap();
    }
}
