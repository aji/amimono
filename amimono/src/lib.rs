//! Amimono is a library-level **modular monolith** framework for building
//! cloud-native applications with Rust that is lightweight and flexible.
//!
//! With Amimono your application is broken up into *components*, which are then
//! collected into *jobs* that can be run as independent workloads. The core
//! Amimono runtime handles service discovery and component dispatch, and
//! optional functionality such as the RPC subsystem makes it easy to define
//! new components that can be used throughout the application.

use std::{net::Ipv4Addr, process};

use crate::{
    config::{Binding, BindingType},
    runtime::ComponentRegistry,
};

pub mod config;
pub mod rpc;
pub mod runtime;

mod macros;

/// The main Amimono entry point.
pub fn entry(cf: config::AppConfig) -> ! {
    if let Err(e) = entry_inner(cf) {
        log::error!("failed to start application: {}", e);
        process::exit(1);
    } else {
        log::warn!("application exited normally");
        process::exit(0);
    }
}

fn entry_inner(cf: config::AppConfig) -> Result<(), String> {
    log::debug!("parse command line args");
    let args = runtime::parse_args()?;

    let mut reg = ComponentRegistry::new();
    for job in cf.jobs() {
        for comp in job.components() {
            log::debug!("init: {} -> {:?}", comp.label, comp.id.0);
            reg.init(comp.label.clone(), comp.id);
        }
    }

    let start_port = 9000;
    let end_port = 9100;
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
                BindingType::HttpFixed(p) => {
                    if start_port < p || p > end_port {
                        panic!(
                            "fixed port {} in reserved range ({}-{})",
                            p, start_port, end_port
                        );
                    }
                    Binding::Http(
                        (Ipv4Addr::LOCALHOST, p).into(),
                        format!("http://localhost:{}", p),
                    )
                }
            };
            log::debug!("binding: {} -> {:?}", comp.label, binding);
            reg.set_binding(&comp.label, binding);
        }
    }

    log::debug!("initializing runtime");
    runtime::init(cf, args, reg);

    log::debug!("launching application");
    runtime::launch()
}
