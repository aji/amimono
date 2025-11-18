//! Amimono is a library-level **modular monolith** framework for building
//! cloud-native applications with Rust that is lightweight and flexible.
//!
//! With Amimono your application is broken up into *components*, which are then
//! collected into *jobs* that can be run as independent workloads. The core
//! Amimono runtime handles service discovery and component dispatch, and
//! optional functionality such as the RPC subsystem makes it easy to define
//! new components that can be used throughout the application.

use std::{net::Ipv4Addr, thread};

use crate::{
    config::{Binding, BindingType},
    runtime::ComponentRegistry,
};

pub mod config;
pub mod rpc;
pub mod runtime;

mod macros;

/// The main Amimono entry point.
///
/// Call this with an `AppConfig` to launch your app.
pub fn entry(cf: config::AppConfig) {
    let mut reg = ComponentRegistry::new();
    for job in cf.jobs() {
        for comp in job.components() {
            log::debug!("init: {} -> {:?}", comp.label, comp.id.0);
            reg.init(comp.label.clone(), comp.id);
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
            log::debug!("binding: {} -> {:?}", comp.label, binding);
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
