//! Amimono is a library-level **modular monolith** framework for building
//! cloud-native applications with Rust that is lightweight and flexible.
//!
//! With Amimono your application is broken up into *components*, which are then
//! collected into *jobs* that can be run as independent workloads. The core
//! Amimono runtime handles service discovery and component dispatch, and
//! optional functionality such as the RPC subsystem makes it easy to define
//! new components that can be used throughout the application.

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, process};

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
        process::exit(0);
    }
}

fn entry_inner(cf: config::AppConfig) -> Result<(), String> {
    log::debug!("parse command line args");
    let args = runtime::parse_args()?;

    log::debug!("initializing component registry");
    let mut reg = ComponentRegistry::new();
    init_components(&cf, &mut reg);

    log::debug!("set component bindings");
    set_bindings(&cf, &mut reg);

    log::debug!("initializing runtime");
    runtime::init(cf, args, reg);

    log::debug!("starting application");
    start()
}

fn init_components(cf: &config::AppConfig, reg: &mut ComponentRegistry) {
    for job in cf.jobs() {
        for comp in job.components() {
            log::debug!("init: {} -> {:?}", comp.label, comp.id.0);
            reg.init(comp.label.clone(), comp.id);
        }
    }
}

fn set_bindings(cf: &config::AppConfig, reg: &mut ComponentRegistry) {
    let start_port = 9000;
    let end_port = 9100;
    let mut port = 9000;
    for job in cf.jobs() {
        for comp in job.components() {
            let binding = match comp.binding {
                BindingType::None => Binding::None,
                BindingType::Http => {
                    let binding = Binding::Http(port);
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
                    Binding::Http(p)
                }
            };
            log::debug!("binding: {} -> {:?}", comp.label, binding);
            reg.set_binding(&comp.label, binding);
        }
    }
}

fn start() -> Result<(), String> {
    use runtime::Action;

    match &runtime::args().action {
        Action::DumpConfig => dump_config(),
        Action::Local => runtime::launch_local(),
        Action::Job(job) => runtime::launch_job(job.as_str()),
    }
}

fn dump_config() -> Result<(), String> {
    let cf = DumpConfig::new();
    let json = serde_json::to_string_pretty(&cf)
        .map_err(|e| format!("failed to serialize config to JSON: {}", e))?;
    println!("{}", json);
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct DumpConfig {
    revision: String,
    jobs: HashMap<String, DumpJob>,
}

#[derive(Serialize, Deserialize)]
struct DumpJob {
    components: HashMap<String, DumpComponent>,
}

#[derive(Serialize, Deserialize)]
struct DumpComponent {
    binding: DumpBinding,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum DumpBinding {
    None,
    Http { port: u16 },
}

impl DumpConfig {
    fn new() -> Self {
        let cf = runtime::config();

        let mut jobs = HashMap::new();

        for job in cf.jobs() {
            let mut components = HashMap::new();
            for comp in job.components() {
                let dump_comp = DumpComponent {
                    binding: match runtime::binding_by_label(&comp.label) {
                        Binding::None => DumpBinding::None,
                        Binding::Http(port) => DumpBinding::Http { port },
                    },
                };
                components.insert(comp.label.clone(), dump_comp);
            }
            jobs.insert(job.label().to_owned(), DumpJob { components });
        }

        DumpConfig {
            revision: cf.revision().to_owned(),
            jobs,
        }
    }
}
