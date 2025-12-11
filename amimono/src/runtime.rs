//! The entry point to the Amimono runtime.
//!
//! The runtime provides access to global information about the application,
//! such as the `AppConfig` and bindings. The runtime is initialized internally.

use std::{net::SocketAddr, path::PathBuf, sync::OnceLock};

use futures::future::BoxFuture;

use crate::{
    cli::Args,
    component::Location,
    config::{AppConfig, ComponentConfig},
    error::{Error, Result},
};

pub(crate) trait RuntimeProvider: Sync + Send + 'static {
    fn discover_running<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        component: &'l str,
    ) -> BoxFuture<'f, Result<Vec<Location>>>;

    fn discover_stable<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        component: &'l str,
    ) -> BoxFuture<'f, Result<Vec<Location>>>;

    fn myself<'f, 'p: 'f, 'l: 'f>(&'p self, component: &'l str) -> BoxFuture<'f, Result<Location>>;

    fn storage<'f, 'p: 'f, 'l: 'f>(&'p self, component: &'l str) -> BoxFuture<'f, Result<PathBuf>>;
}

pub(crate) struct NoopRuntime;

impl RuntimeProvider for NoopRuntime {
    fn discover_running<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        _component: &'l str,
    ) -> BoxFuture<'f, Result<Vec<Location>>> {
        Box::pin(async { Err("discover_running() called on noop runtime")? })
    }

    fn discover_stable<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        _component: &'l str,
    ) -> BoxFuture<'f, Result<Vec<Location>>> {
        Box::pin(async { Err("discover_stable() called on noop runtime")? })
    }

    fn myself<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        _component: &'l str,
    ) -> BoxFuture<'f, Result<Location>> {
        Box::pin(async { Err("myself() called on noop runtime")? })
    }

    fn storage<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        _component: &'l str,
    ) -> BoxFuture<'f, Result<PathBuf>> {
        Box::pin(async { Err("storage() called on noop runtime")? })
    }
}

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

struct Runtime {
    cf: AppConfig,
    args: Args,
    provider: Box<dyn RuntimeProvider>,
}

pub(crate) fn init(cf: AppConfig, args: Args, provider: Box<dyn RuntimeProvider>) {
    let rt = Runtime { cf, args, provider };
    RUNTIME.set(rt).ok().expect("runtime already initialized");
}

fn get() -> &'static Runtime {
    RUNTIME.get().expect("runtime not initialized")
}

/// Get the `AppConfig` used to start the application.
pub fn config() -> &'static AppConfig {
    &get().cf
}

pub(crate) fn provider() -> &'static dyn RuntimeProvider {
    &*get().provider
}

pub(crate) fn args() -> &'static Args {
    &get().args
}

/// Get a SockAddr to bind to for a given port
pub fn to_addr(port: u16) -> SocketAddr {
    match &args().bind {
        Some(s) => format!("{s}:{port}")
            .parse()
            .expect("could not parse into sockaddr"),
        None => ([0, 0, 0, 0], port)
            .try_into()
            .expect("could not try_into sockaddr"),
    }
}

async fn launch_comps(to_launch: Vec<&ComponentConfig>) -> Result<()> {
    let joins = to_launch
        .into_iter()
        .map(|comp| {
            log::debug!("spawn {}", comp.label);
            tokio::spawn((comp.entry)())
        })
        .collect::<Vec<_>>();

    log::info!("components started");
    for join in joins {
        join.await
            .map_err(|e| format!("component task failed: {}", e))?;
    }

    Ok(())
}

pub(crate) async fn launch_local() -> Result<()> {
    launch_comps(config().jobs().flat_map(|j| j.components()).collect()).await
}

pub(crate) async fn launch_job(job: &str) -> Result<()> {
    match config().job(job) {
        Some(j) => launch_comps(j.components().collect()).await,
        None => Err(format!("no such job: {}", job))?,
    }
}

pub(crate) async fn launch_tool(tool: &str) -> Result<()> {
    match config().tool(tool) {
        Some(t) => {
            log::info!("starting tool {tool}");
            t.entry.entry().await;
            Ok(())
        }
        None => {
            let tools = config()
                .tools()
                .map(|x| x.label.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            log::error!("no such tool {tool}");
            log::info!("available tools: {}", tools);
            Err(Error::User(format!("no such tool {tool}")))
        }
    }
}
