//! The entry point to the Amimono runtime.
//!
//! The runtime provides access to global information about the application,
//! such as the `AppConfig` and bindings. The runtime is initialized internally.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    net::SocketAddr,
    path::PathBuf,
    sync::OnceLock,
};

use futures::future::BoxFuture;

use crate::config::AppConfig;

/// Types that can be used as keys in the Amimono runtime.
///
/// Types implementing this trait are used by the runtime as keys for things
/// such as accessing a component's bindings. (See [`binding`].)
pub trait Component: 'static {
    /// The component's "instance" type.
    ///
    /// Each component registers a value of type `Instance` with the runtime.
    /// Other components can retrieve this value. If a component does not offer
    /// an in-process way of interacting with colocated components, it can set
    /// this to something like `()`.
    ///
    /// Note that all components must register an instance with the runtime,
    /// even if they are not running.
    ///
    /// For an example of a non-trivial usage of `Instance`, see
    /// [`RpcComponent`](crate::rpc::RpcComponent)
    type Instance: Sync + Send;

    fn id() -> ComponentId {
        ComponentId(TypeId::of::<Self>())
    }
}

/// An opaque identifier for a `Component` type.
#[derive(Copy, Clone)]
pub struct ComponentId(pub(crate) TypeId);

/// A string representing a network location.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Location {
    /// A hostname or IP address that is only temporarily valid.
    Ephemeral(String),
    /// A hostname or IP address that can be used long term.
    Stable(String),
}

impl Location {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Location::Ephemeral(s) => Some(s.as_str()),
            Location::Stable(s) => Some(s.as_str()),
        }
    }
}

pub type RuntimeResult<T> = Result<T, &'static str>;

pub(crate) trait RuntimeProvider: Sync + Send + 'static {
    fn discover<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        component: &'l str,
    ) -> BoxFuture<'f, RuntimeResult<Vec<Location>>>;

    fn myself<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        component: &'l str,
    ) -> BoxFuture<'f, RuntimeResult<Location>>;

    fn storage<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        component: &'l str,
    ) -> BoxFuture<'f, RuntimeResult<PathBuf>>;
}

pub(crate) struct NoopRuntime;

impl RuntimeProvider for NoopRuntime {
    fn discover<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        _component: &'l str,
    ) -> BoxFuture<'f, RuntimeResult<Vec<Location>>> {
        Box::pin(async { Err("discover() called on noop runtime") })
    }

    fn myself<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        _component: &'l str,
    ) -> BoxFuture<'f, RuntimeResult<Location>> {
        Box::pin(async { Err("myself() called on noop runtime") })
    }

    fn storage<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        _component: &'l str,
    ) -> BoxFuture<'f, RuntimeResult<PathBuf>> {
        Box::pin(async { Err("storage() called on noop runtime") })
    }
}

pub(crate) struct ComponentRegistry {
    labels: HashMap<String, TypeId>,
    components: HashMap<TypeId, ComponentInfo>,
}

struct ComponentInfo {
    label: String,
    instance: OnceLock<Box<dyn Any + Sync + Send>>,
}

impl ComponentRegistry {
    pub fn new() -> ComponentRegistry {
        ComponentRegistry {
            labels: HashMap::new(),
            components: HashMap::new(),
        }
    }

    pub fn init<S: AsRef<str>>(&mut self, label: S, ComponentId(ty): ComponentId) {
        let label = label.as_ref().to_owned();
        let info = ComponentInfo {
            label: label.clone(),
            instance: OnceLock::new(),
        };
        self.labels.insert(label, ty);
        self.components.insert(ty, info);
    }

    fn by_type<C: Component>(&self) -> Option<&ComponentInfo> {
        self.components.get(&TypeId::of::<C>())
    }
}

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

struct Runtime {
    cf: AppConfig,
    args: Args,
    provider: Box<dyn RuntimeProvider>,
    registry: ComponentRegistry,
}

pub(crate) fn init(
    cf: AppConfig,
    args: Args,
    provider: Box<dyn RuntimeProvider>,
    registry: ComponentRegistry,
) {
    let rt = Runtime {
        cf,
        args,
        provider,
        registry,
    };
    RUNTIME.set(rt).ok().expect("runtime already initialized");
}

fn get() -> &'static Runtime {
    RUNTIME.get().expect("runtime not initialized")
}

fn registry() -> &'static ComponentRegistry {
    &get().registry
}

fn provider() -> &'static dyn RuntimeProvider {
    &*get().provider
}

pub(crate) fn args() -> &'static Args {
    &get().args
}

/// Get the `AppConfig` used to start the application.
pub fn config() -> &'static AppConfig {
    &get().cf
}

/// Get a SockAddr to bind to for a given point
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

/// Get a component's string label.
pub fn label<C: Component>() -> &'static str {
    registry()
        .by_type::<C>()
        .expect("component type not registered")
        .label
        .as_str()
}

/// Get a component's job label
pub fn job_label<C: Component>() -> &'static str {
    get()
        .cf
        .component_job(label::<C>())
        .expect("component not in config")
}

/// Set a component's instance data.
///
/// This should only be called once for each component within a process.
pub fn set_instance<C: Component>(instance: C::Instance) {
    assert!(is_local::<C>());
    registry()
        .by_type::<C>()
        .expect("component type not registered")
        .instance
        .set(Box::new(instance))
        .ok()
        .expect("component instance already set");
}

/// Get a component's instance data.
///
/// This function will block until the corresponding [`set_instance`] call.
pub fn get_instance<C: Component>() -> &'static C::Instance {
    assert!(is_local::<C>());
    registry()
        .by_type::<C>()
        .expect("component type not registered")
        .instance
        .wait()
        .downcast_ref()
        .expect("instance downcast failed")
}

/// Discover a component's locations
pub async fn discover<C: Component>() -> RuntimeResult<Vec<Location>> {
    discover_by_label(label::<C>()).await
}

/// Get a component's own location
pub async fn myself<C: Component>() -> RuntimeResult<Location> {
    assert!(is_local::<C>());
    provider().myself(label::<C>()).await
}

/// Discover a component's locations by its label
pub async fn discover_by_label<S: AsRef<str>>(label: S) -> RuntimeResult<Vec<Location>> {
    provider().discover(label.as_ref()).await
}

/// Get a component's storage path
pub async fn storage<C: Component>() -> RuntimeResult<PathBuf> {
    let label = label::<C>();
    let component = config().component(label).ok_or("component not in config")?;
    if component.is_stateful {
        provider().storage(label).await
    } else {
        Err("component is not stateful")
    }
}

/// Determine whether a target component is running locally
pub fn is_local<C: Component>() -> bool {
    match &args().action {
        Action::DumpConfig => panic!("is_local called in dump-config mode"),
        Action::Local => true,
        Action::Job(target_label) => job_label::<C>() == target_label,
    }
}

pub(crate) struct Args {
    pub action: Action,
    pub bind: Option<String>,
    pub r#static: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    DumpConfig,
    Local,
    Job(String),
}

pub(crate) fn parse_args() -> Result<Args, String> {
    use clap::{Arg, ArgAction, Command};

    let m = Command::new("amimono")
        .arg(
            Arg::new("dump-config")
                .long("dump-config")
                .action(ArgAction::SetTrue)
                .help("Dump the application configuration and exit"),
        )
        .arg(
            Arg::new("local")
                .long("local")
                .action(ArgAction::SetTrue)
                .help("Run in local mode"),
        )
        .arg(
            Arg::new("job")
                .long("job")
                .action(ArgAction::Set)
                .help("The job to run"),
        )
        .arg(
            Arg::new("static")
                .long("static")
                .action(ArgAction::Set)
                .help("The static config root to use. Forces the static runtime."),
        )
        .arg(
            Arg::new("bind")
                .long("bind")
                .action(ArgAction::Set)
                .help("The IP address to bind to."),
        )
        .get_matches();

    let action = [
        m.get_flag("dump-config").then_some(Action::DumpConfig),
        m.get_flag("local").then_some(Action::Local),
        m.get_one::<String>("job").map(|j| Action::Job(j.clone())),
    ]
    .into_iter()
    .filter(|x| x.is_some())
    .reduce(|_, _| None)
    .flatten()
    .ok_or("must specify exactly one of --local, --job <job>, or --dump-config")?;

    let bind = m.get_one::<String>("bind").cloned();
    let r#static = m.get_one::<String>("static").cloned();

    Ok(Args {
        action,
        bind,
        r#static,
    })
}

pub(crate) async fn launch_local() -> Result<(), String> {
    let mut joins = Vec::new();
    for job in config().jobs() {
        for comp in job.components() {
            log::debug!("spawn {}", comp.label);
            joins.push(tokio::spawn((comp.entry)()));
        }
    }

    log::info!("components started");
    for join in joins {
        join.await
            .map_err(|e| format!("component task failed: {}", e))?;
    }

    Ok(())
}

pub(crate) async fn launch_job(job: &str) -> Result<(), String> {
    let job = match config().job(job) {
        Some(j) => j,
        None => return Err(format!("no such job: {}", job)),
    };

    let mut joins = Vec::new();
    for comp in job.components() {
        log::debug!("spawn {}", comp.label);
        joins.push(tokio::spawn((comp.entry)()));
    }

    log::info!("components started");
    for join in joins {
        join.await
            .map_err(|e| format!("component task failed: {}", e))?;
    }

    Ok(())
}
