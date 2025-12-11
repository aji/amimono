//! Amimono is a library-level **modular monolith** framework for building
//! cloud-native applications with Rust that is lightweight and flexible.
//!
//! With Amimono your application is broken up into *components*, which are then
//! collected into *jobs* that can be run as independent workloads. The core
//! Amimono runtime handles service discovery and component dispatch, and
//! optional functionality such as the RPC subsystem makes it easy to define
//! new components that can be used throughout the application.

use amimono_schemas::{DumpComponent, DumpConfig, DumpJob};
use std::{collections::HashMap, path::PathBuf, process};

use crate::{
    component::Location, local::LocalRuntime, runtime::NoopRuntime, r#static::StaticRuntime,
};

pub mod component;
pub mod config;
pub mod retry;
pub mod rpc;
pub mod runtime;

pub(crate) mod cli;
pub(crate) mod error;
pub(crate) mod k8s;
pub(crate) mod local;
pub(crate) mod r#static;
pub(crate) mod util;

pub use error::{AppError, AppResult, Error, Result};

pub use futures::future::BoxFuture;

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
async fn entry_inner(cf: config::AppConfig) -> Result<()> {
    log::debug!("parse command line args");
    let args = cli::parse_args()?;

    log::debug!("initializing runtime provider");
    let provider = init_runtime_provider(&cf, &args).await;

    log::debug!("initializing runtime");
    runtime::init(cf, args, provider);

    log::debug!("starting application");
    start().await
}

async fn init_runtime_provider(
    _cf: &config::AppConfig,
    args: &cli::Args,
) -> Box<dyn runtime::RuntimeProvider> {
    match args.action {
        cli::Action::DumpConfig => Box::new(NoopRuntime),
        cli::Action::Local => {
            let dir = match std::env::var("CARGO_MANIFEST_DIR") {
                Ok(dir) => dir,
                Err(_) => {
                    log::warn!("--local outside of cargo! local runtime using current directory");
                    ".".to_owned()
                }
            };
            Box::new(LocalRuntime::new(dir))
        }
        _ => {
            if let Some(s) = &args.r#static {
                let myself = match &args.bind {
                    Some(x) => Location::stable(x.clone()),
                    None => {
                        log::error!("static runtime requires --bind");
                        panic!();
                    }
                };
                log::debug!("starting static runtime as {myself:?} in {s}");
                Box::new(StaticRuntime::open(PathBuf::from(s), myself))
            } else if let Ok(config) = kube::config::Config::incluster_env() {
                log::debug!("detected Kubernetes environment");
                Box::new(k8s::K8sRuntime::new("default".to_owned(), config).await)
            } else if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
                log::debug!("detected local development environment");
                Box::new(LocalRuntime::new(dir))
            } else {
                log::warn!("could not detect running environment, falling back to noop discovery");
                Box::new(NoopRuntime)
            }
        }
    }
}

async fn start() -> Result<()> {
    use cli::Action;

    match &runtime::args().action {
        Action::DumpConfig => dump_config(),
        Action::Local => runtime::launch_local().await,
        Action::Job(job) => runtime::launch_job(job.as_str()).await,
        Action::Tool(tool) => runtime::launch_tool(tool.as_str()).await,
    }
}

fn dump_config() -> Result<()> {
    let cf = {
        let cf = runtime::config();

        let mut jobs = HashMap::new();

        for job in cf.jobs() {
            let mut components = HashMap::new();
            for comp in job.components() {
                let dump_comp = DumpComponent {
                    is_stateful: comp.is_stateful,
                    ports: comp.ports.clone(),
                };
                components.insert(comp.label.clone(), dump_comp);
            }
            jobs.insert(
                job.label().to_owned(),
                DumpJob {
                    is_stateful: job.is_stateful(),
                    components,
                },
            );
        }

        DumpConfig {
            revision: cf.revision().to_owned(),
            jobs,
        }
    };

    let json = serde_json::to_string_pretty(&cf)
        .map_err(|e| format!("failed to serialize config to JSON: {}", e))?;
    println!("{}", json);
    Ok(())
}
