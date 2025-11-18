use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::OnceLock,
};

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

    pub fn set_label<C: Component>(&mut self, label: String) {
        self.labels.insert(TypeId::of::<C>(), label);
    }

    pub fn register<C: Component>(&mut self, instance: C::Instance) {
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

static REGISTRY: OnceLock<ComponentRegistry> = OnceLock::new();

pub fn set_registry(reg: ComponentRegistry) {
    REGISTRY
        .set(reg)
        .ok()
        .expect("REGISTRY is already initialized")
}

pub fn label<C: Component>() -> Option<&'static str> {
    REGISTRY
        .get()
        .expect("REGISTRY is not initialized")
        .label::<C>()
}

pub fn instance<C: Component>() -> Option<&'static C::Instance> {
    REGISTRY
        .get()
        .expect("REGISTRY is not initialized")
        .instance::<C>()
}
