use std::{
    any::{Any, TypeId},
    borrow::Borrow,
    fmt,
    path::PathBuf,
};

use futures::future::BoxFuture;
use tokio::sync::SetOnce;

use crate::{
    cli,
    config::{ComponentConfig, JobBuilder},
    error::Result,
    runtime,
    util::StaticHashMap,
};

/// A string representing a network location.
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct Location<A = String> {
    ephemeral: bool,
    addr: A,
}

impl<A> Location<A> {
    pub fn emphemeral(addr: A) -> Location<A> {
        Location {
            ephemeral: true,
            addr,
        }
    }

    pub fn stable(addr: A) -> Location<A> {
        Location {
            ephemeral: false,
            addr,
        }
    }

    pub fn is_ephemeral(&self) -> bool {
        self.ephemeral
    }

    pub fn is_stable(&self) -> bool {
        !self.ephemeral
    }

    pub fn as_ephemeral(self) -> std::result::Result<A, A> {
        match self.ephemeral {
            true => Ok(self.addr),
            false => Err(self.addr),
        }
    }

    pub fn as_stable(self) -> std::result::Result<A, A> {
        match self.ephemeral {
            true => Err(self.addr),
            false => Ok(self.addr),
        }
    }

    pub fn addr<B>(&self) -> &B
    where
        B: ?Sized,
        A: Borrow<B>,
    {
        self.addr.borrow()
    }

    pub fn into_addr(self) -> A {
        self.addr
    }
}

impl Location<String> {
    pub fn borrow(&'_ self) -> Location<&'_ str> {
        Location {
            ephemeral: self.ephemeral,
            addr: self.addr.as_str(),
        }
    }
}

impl<'l> Location<&'l str> {
    pub fn into_owned(self) -> Location<String> {
        Location {
            ephemeral: self.ephemeral,
            addr: self.addr.to_owned(),
        }
    }
}

impl<L: fmt::Debug> fmt::Debug for Location<L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.ephemeral {
            true => "ephemeral",
            false => "stable",
        };
        write!(f, "Location::{}({:?})", kind, self.addr)
    }
}

/// An opaque identifier for a `ComponentKind`.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ComponentKindId(TypeId);

/// A type acting as a key in the Amimono runtime.
pub trait ComponentKind: 'static {
    /// This component's instance type. Implementations of this component must
    /// be able to provide a value of this type for the rest of the process to
    /// use.
    type Instance: Clone + Send + Sync + 'static;

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

    /// Provided method to get this component kind's ID
    fn id() -> ComponentKindId {
        ComponentKindId(TypeId::of::<Self>())
    }

    /// Provided method to get a future that resolves to this component's
    /// instance. It will resolve immediately with `None` if the component is
    /// not running within the same process.
    fn instance() -> Option<impl Future<Output = Self::Instance> + Send> {
        if Self::is_local() {
            let cell = INSTANCES.get_or_insert(Self::LABEL);
            Some(async move {
                cell.wait()
                    .await
                    .downcast_ref::<Self::Instance>()
                    .expect("downcast failed")
                    .clone()
            })
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

/// A trait for types that implement a `Component`.
///
/// This is a separate trait because components and their implementations may
/// not live in the same crate. An application can have at most one
/// `ComponentImpl` per `Component`, but different applications can use
/// different `ComponentImpl`s for the same component.
pub trait Component: Sized + 'static {
    /// The `Component` this type implements
    type Kind: ComponentKind;

    /// The component impl's entry point.
    ///
    /// The provided `set_instance` function should be called with an instance
    /// value. The future returned by `set_instance` will resolve when all
    /// components in the same job have called their corresponding
    /// `set_instance` callbacks.
    fn main<F>(set_instance: F) -> impl Future<Output = ()> + Send
    where
        F: FnOnce(<Self::Kind as ComponentKind>::Instance) -> BoxFuture<'static, ()> + Send;

    /// Provided method to get this component's storage path. It's assumed this
    /// is only called from the implementation while it's running, and will
    /// panic if the component is not local or stateful.
    fn storage() -> impl Future<Output = Result<PathBuf>> + Send {
        runtime::provider().storage(Self::Kind::LABEL)
    }

    /// Provided method to install this component implementation in a job config.
    fn installer(job: &mut JobBuilder) {
        job.add_component(ComponentConfig {
            id: Self::Kind::id(),
            label: Self::Kind::LABEL.to_owned(),
            ports: Self::Kind::PORTS.to_owned(),
            is_stateful: Self::Kind::STORAGE.is_some(),
            entry: component_impl_entry::<Self>,
        });
    }
}

type InstanceCell = SetOnce<Box<dyn Any + Send + Sync>>;

static INSTANCES: StaticHashMap<&'static str, InstanceCell> = StaticHashMap::new();

fn component_impl_entry<C: Component>() -> BoxFuture<'static, ()> {
    Box::pin(C::main(|instance| {
        Box::pin(async {
            INSTANCES
                .get_or_insert(C::Kind::LABEL)
                .set(Box::new(instance))
                .expect("SetOnce::set() failed!");
        })
    }))
}
