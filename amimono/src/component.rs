use std::{
    any::{Any, TypeId},
    collections::HashMap,
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
