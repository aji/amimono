//! Amimono is a library-level **modular monolith** framework for building
//! cloud-native applications with Rust that is lightweight and flexible.
//!
//! With Amimono your application is broken up into *components*, which are then
//! collected into *jobs* that can be run as independent workloads. The core
//! Amimono runtime handles service discovery and component dispatch, and
//! optional functionality such as the RPC subsystem makes it easy to define
//! new components that can be used throughout the application.

use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, process};

use crate::{
    config::{Binding, BindingType},
    runtime::{ComponentRegistry, DiscoveryProvider, Location},
};

pub mod config;
pub mod k8s;
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

#[tokio::main]
async fn entry_inner(cf: config::AppConfig) -> Result<(), String> {
    log::debug!("parse command line args");
    let args = runtime::parse_args()?;

    log::debug!("initializing component registry");
    let mut reg = ComponentRegistry::new();
    init_components(&cf, &mut reg);

    log::debug!("set component bindings");
    set_bindings(&cf, &mut reg);

    log::debug!("initializing discovery provider");
    let discovery = init_discovery(&cf, &args).await;

    log::debug!("initializing runtime");
    runtime::init(cf, args, discovery, reg);

    log::debug!("starting application");
    start().await
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

struct NoopDiscovery;

impl DiscoveryProvider for NoopDiscovery {
    fn discover(&'_ self, _component: &str) -> BoxFuture<'_, Location> {
        Box::pin(async { Location::None })
    }
}

struct LocalDiscovery;

impl runtime::DiscoveryProvider for LocalDiscovery {
    fn discover(&'_ self, label: &str) -> BoxFuture<'_, Location> {
        let binding = runtime::binding_by_label(label);
        let res = match binding {
            Binding::None => Location::None,
            Binding::Http(port) => {
                let url = format!("http://localhost:{}", port);
                Location::Http(url)
            }
        };
        Box::pin(async { res })
    }
}

async fn init_discovery(
    _cf: &config::AppConfig,
    args: &runtime::Args,
) -> Box<dyn runtime::DiscoveryProvider> {
    match args.action {
        runtime::Action::DumpConfig => Box::new(NoopDiscovery),
        runtime::Action::Local => Box::new(LocalDiscovery),
        runtime::Action::Job(_) => {
            if let Ok(config) = kube::config::Config::incluster_env() {
                log::debug!("detected Kubernetes environment");
                Box::new(k8s::K8sDiscovery::new("default".to_owned(), config).await)
            } else {
                log::warn!("could not detect running environment, falling back to noop discovery");
                Box::new(NoopDiscovery)
            }
        }
    }
}

async fn start() -> Result<(), String> {
    use runtime::Action;

    match &runtime::args().action {
        Action::DumpConfig => dump_config(),
        Action::Local => runtime::launch_local().await,
        Action::Job(job) => runtime::launch_job(job.as_str()).await,
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
