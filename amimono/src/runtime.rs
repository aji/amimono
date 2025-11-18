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
    labels: HashMap<String, TypeId>,
    components: HashMap<TypeId, ComponentInfo>,
}

struct ComponentInfo {
    label: String,
    instance: Box<dyn Any + Sync + Send>,
    binding: Binding,
}

impl ComponentRegistry {
    pub fn new() -> ComponentRegistry {
        ComponentRegistry {
            labels: HashMap::new(),
            components: HashMap::new(),
        }
    }

    pub fn register<C: Component>(&mut self, label: String, instance: C::Instance) {
        let ty = TypeId::of::<C>();
        let info = ComponentInfo {
            label: label.clone(),
            instance: Box::new(instance),
            binding: Binding::None,
        };
        self.labels.insert(label, ty);
        self.components.insert(ty, info);
    }

    pub fn set_binding<S: AsRef<str>>(&mut self, label: S, binding: Binding) {
        self.by_label_mut(label)
            .expect("component not registered")
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

pub fn init(cf: AppConfig, registry: ComponentRegistry) {
    let rt = Runtime { cf, registry };
    RUNTIME.set(rt).ok().expect("runtime already initialized");
}

fn get() -> &'static Runtime {
    RUNTIME.get().expect("runtime not initialized")
}

fn registry() -> &'static ComponentRegistry {
    &get().registry
}

pub fn config() -> &'static AppConfig {
    &get().cf
}

pub fn label<C: Component>() -> Option<&'static str> {
    registry().by_type::<C>().map(|x| x.label.as_str())
}

pub fn instance<C: Component>() -> Option<&'static C::Instance> {
    registry()
        .by_type::<C>()
        .and_then(|x| x.instance.downcast_ref())
}

pub fn binding<C: Component>() -> Option<Binding> {
    registry().by_type::<C>().map(|x| x.binding.clone())
}
