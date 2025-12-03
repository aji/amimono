use std::{
    any::{Any, TypeId},
    collections::HashMap,
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

use futures::future::BoxFuture;

use crate::{
    cli,
    config::{ComponentConfig, JobBuilder},
    error::Result,
    runtime,
};

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

/// An opaque identifier for a `Component` type.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ComponentId(TypeId);

/// A type acting as a key in the Amimono runtime.
pub trait Component: 'static {
    /// This component's instance type. Implementations of this component must
    /// be able to provide a value of this type for the rest of the process to
    /// use.
    type Instance: Clone + Send + 'static;

    /// A globally unique string identifier for this component.
    const LABEL: &'static str;

    /// A list of ports this component will bind. This is metadata used for
    /// things like generating container configs. Components within the same job
    /// can have the same port numbers here, as long as they have a mechanism
    /// for sharing the port.
    const PORTS: &'static [u16] = &[];

    /// Indicates how much disk storage is requested by this component, in
    /// bytes. If `None`, the component is assumed to be stateless.
    const STORAGE: Option<usize> = None;

    /// Provided method to get this component's ID
    fn id() -> ComponentId {
        ComponentId(TypeId::of::<Self>())
    }

    /// Provided method to get this component's instance. This will be `None` if
    /// the component is not running within the same process.
    ///
    /// This may panic if the instance is not yet registered.
    fn instance() -> Option<Self::Instance> {
        if Self::is_local() {
            let lock = INSTANCES.lock().expect("INSTANCES lock poisoned");
            let instance: &Self::Instance = lock
                .get(Self::LABEL)
                .expect("instance not yet registered")
                .downcast_ref::<Self::Instance>()
                .expect("downcast failed");
            Some(instance.clone())
        } else {
            None
        }
    }

    /// Provided method to check if the component is running in the same process.
    fn is_local() -> bool {
        match &runtime::args().action {
            cli::Action::DumpConfig => panic!(),
            cli::Action::Local => true,
            cli::Action::Job(j) => runtime::config().component_job(Self::LABEL) == Some(j),
        }
    }

    /// Provided method to get the network location of the current process.
    fn myself() -> impl Future<Output = Result<Location>> + Send {
        runtime::provider().myself(Self::LABEL)
    }

    /// Provided method to get the network locations where this component is
    /// expected to be currently running, although there is no guarantee that
    /// requests to that location will succeed.
    fn discover_running() -> impl Future<Output = Result<Vec<Location>>> + Send {
        runtime::provider().discover_running(Self::LABEL)
    }

    /// Provided method to get the network locations where this component is
    /// stably placed. The component may not be currently running at that
    /// location, however. In the steady state, the `Stable` locations returned
    /// by `discover_running` will be a subset of this list.
    fn discover_stable() -> impl Future<Output = Result<Vec<Location>>> + Send {
        runtime::provider().discover_stable(Self::LABEL)
    }
}

/// A trait for types that implement a `Component`
///
/// This is a separate trait because components and their implementations may
/// not live in the same crate. An application can at most one `ComponentImpl`
/// per `Component`, but different applications can use different
/// `ComponentImpl`s for the same component.
pub trait ComponentImpl: Sized + 'static {
    /// The `Component` this type implements
    type Component: Component;

    /// The component impl's entry point.
    ///
    /// The provided `set_instance` function should be called with an instance
    /// value. The future returned by `set_instance` will resolve when all
    /// components in the same job have called their corresponding
    /// `set_instance` callbacks.
    fn main<F>(set_instance: F) -> impl Future<Output = ()> + Send
    where
        F: FnOnce(<Self::Component as Component>::Instance) -> BoxFuture<'static, ()> + Send;

    /// Provided method to get this component's storage path. It's assumed this
    /// is only called from the implementation while it's running, and will
    /// panic if the component is not local or stateful.
    fn storage() -> impl Future<Output = Result<PathBuf>> + Send {
        runtime::provider().storage(Self::Component::LABEL)
    }

    /// Provided method to install this component implementation in a job config.
    fn installer(job: &mut JobBuilder) {
        job.add_component(ComponentConfig {
            id: Self::Component::id(),
            label: Self::Component::LABEL.to_owned(),
            ports: Self::Component::PORTS.to_owned(),
            is_stateful: Self::Component::STORAGE.is_some(),
            entry: component_impl_entry::<Self>,
        });
    }
}

static INSTANCES: LazyLock<Mutex<HashMap<&'static str, Box<dyn Any + Send>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn component_impl_entry<C: ComponentImpl>(
    barrier: &'static tokio::sync::Barrier,
) -> BoxFuture<'static, ()> {
    Box::pin(C::main(|instance| {
        Box::pin(async {
            let _ = {
                INSTANCES
                    .lock()
                    .expect("INSTANCES lock poisoned")
                    .insert(C::Component::LABEL, Box::new(instance));
            };
            barrier.wait().await;
        })
    }))
}
