use std::sync::OnceLock;

use crate::config::AppConfig;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub struct Runtime {
    cf: AppConfig,
}

pub fn init(cf: AppConfig) {
    let rt = Runtime { cf };
    RUNTIME.set(rt).ok().expect("runtime already initialized");
}

fn get() -> &'static Runtime {
    RUNTIME.get().expect("runtime not initialized")
}

pub fn config() -> &'static AppConfig {
    &get().cf
}
