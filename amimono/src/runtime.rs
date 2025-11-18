//! The entry point to the Amimono runtime.
//!
//! The runtime provides access to global information about the application,
//! such as the `AppConfig` and bindings. The runtime is initialized internally.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::OnceLock,
};

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

    fn by_label_mut<S: AsRef<str>>(&mut self, label: S) -> Option<&mut ComponentInfo> {
        self.labels
            .get(label.as_ref())
            .and_then(|ty| self.components.get_mut(ty))
    }
}

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

struct Runtime {
    cf: AppConfig,
    registry: ComponentRegistry,
}

pub(crate) fn init(cf: AppConfig, registry: ComponentRegistry) {
    let rt = Runtime { cf, registry };
    RUNTIME.set(rt).ok().expect("runtime already initialized");
}

fn get() -> &'static Runtime {
    RUNTIME.get().expect("runtime not initialized")
}

fn registry() -> &'static ComponentRegistry {
    &get().registry
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
