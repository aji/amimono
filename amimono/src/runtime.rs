//! The entry point to the Amimono runtime.
//!
//! The runtime provides access to global information about the application,
//! such as the `AppConfig` and bindings. The runtime is initialized internally.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::OnceLock,
};

use futures::future::BoxFuture;

use crate::config::{AppConfig, Binding};

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

/// A string representing a physical location.
#[derive(Clone, Hash, PartialEq, Eq)]
pub enum Location {
    None,
    Http(String),
}

pub(crate) trait DiscoveryProvider: Sync + Send + 'static {
    fn discover(&'_ self, component: &'static str) -> BoxFuture<'_, Location>;
}

pub(crate) struct ComponentRegistry {
    labels: HashMap<String, TypeId>,
    components: HashMap<TypeId, ComponentInfo>,
}

struct ComponentInfo {
    label: String,
    instance: OnceLock<Box<dyn Any + Sync + Send>>,
    binding: Binding,
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
            binding: Binding::None,
        };
        self.labels.insert(label, ty);
        self.components.insert(ty, info);
    }

    pub fn set_binding<S: AsRef<str>>(&mut self, label: S, binding: Binding) {
        self.by_label_mut(label)
            .expect("component not initialized")
            .binding = binding;
    }

    fn by_type<C: Component>(&self) -> Option<&ComponentInfo> {
        self.components.get(&TypeId::of::<C>())
    }

    fn by_label<S: AsRef<str>>(&self, label: S) -> Option<&ComponentInfo> {
        self.labels
            .get(label.as_ref())
            .and_then(|ty| self.components.get(ty))
    }

    fn by_label_mut<S: AsRef<str>>(&mut self, label: S) -> Option<&mut ComponentInfo> {
        self.labels
            .get(label.as_ref())
            .and_then(|ty| self.components.get_mut(ty))
    }
}

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

struct Runtime {
    cf: AppConfig,
    args: Args,
    discovery: Box<dyn DiscoveryProvider>,
    registry: ComponentRegistry,
}

pub(crate) fn init(
    cf: AppConfig,
    args: Args,
    discovery: Box<dyn DiscoveryProvider>,
    registry: ComponentRegistry,
) {
    let rt = Runtime {
        cf,
        args,
        discovery,
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

pub(crate) fn args() -> &'static Args {
    &get().args
}

/// Get the `AppConfig` used to start the application.
pub fn config() -> &'static AppConfig {
    &get().cf
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
    registry()
        .by_type::<C>()
        .expect("component type not registered")
        .instance
        .wait()
        .downcast_ref()
        .expect("instance downcast failed")
}

/// Get a component's binding.
pub fn binding<C: Component>() -> Binding {
    registry()
        .by_type::<C>()
        .expect("component type not registered")
        .binding
        .clone()
}

pub fn binding_by_label<S: AsRef<str>>(label: S) -> Binding {
    registry()
        .by_label(label)
        .expect("component not initialized")
        .binding
        .clone()
}

/// Discover a component's location
pub async fn discover<C: Component>() -> Location {
    get().discovery.discover(label::<C>()).await
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

    Ok(Args { action })
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
