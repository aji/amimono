use std::sync::OnceLock;

use crate::{
    component::{Component, ComponentRegistry},
    config::AppConfig,
};

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub struct Runtime {
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
