use std::{net::Ipv4Addr, thread};

use crate::{
    config::{Binding, BindingType},
    runtime::ComponentRegistry,
};

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

    let mut port = 9000;
    for job in cf.jobs() {
        for comp in job.components() {
            let binding = match comp.binding {
                BindingType::None => Binding::None,
                BindingType::Http => {
                    let binding = Binding::Http(
                        (Ipv4Addr::LOCALHOST, port).into(),
                        format!("http://localhost:{}", port),
                    );
                    port += 1;
                    binding
                }
            };
            log::debug!("allocating {:?} to {}", binding, comp.label);
            reg.set_binding(&comp.label, binding);
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
