use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::OnceLock,
};

use crate::config::{AppConfig, Binding};

pub trait Component: 'static {
    type Instance: Sync + Send;
}

pub struct ComponentRegistry {
    labels: HashMap<TypeId, String>,
    instances: HashMap<TypeId, Box<dyn Any + Sync + Send>>,
}

impl ComponentRegistry {
    pub fn new() -> ComponentRegistry {
        ComponentRegistry {
            labels: HashMap::new(),
            instances: HashMap::new(),
        }
    }

    pub fn register<C: Component>(&mut self, label: String, instance: C::Instance) {
        self.labels.insert(TypeId::of::<C>(), label);
        self.instances.insert(TypeId::of::<C>(), Box::new(instance));
    }

    pub fn label<C: Component>(&self) -> Option<&str> {
        self.labels.get(&TypeId::of::<C>()).map(|x| x.as_str())
    }

    pub fn instance<C: Component>(&self) -> Option<&C::Instance> {
        self.instances
            .get(&TypeId::of::<C>())
            .and_then(|x| x.downcast_ref())
    }
}

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

struct Runtime {
    cf: AppConfig,
    registry: ComponentRegistry,
}

pub fn init(cf: AppConfig, registry: ComponentRegistry) {
    let rt = Runtime { cf, registry };
    RUNTIME.set(rt).ok().expect("runtime already initialized");
}

fn get() -> &'static Runtime {
    RUNTIME.get().expect("runtime not initialized")
}

pub fn config() -> &'static AppConfig {
    &get().cf
}

pub fn label<C: Component>() -> Option<&'static str> {
    get().registry.label::<C>()
}

pub fn instance<C: Component>() -> Option<&'static C::Instance> {
    get().registry.instance::<C>()
}

pub fn binding<C: Component>() -> Option<Binding> {
    None
}
