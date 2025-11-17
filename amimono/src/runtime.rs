use std::{any::Any, collections::HashMap, sync::Arc};

use tokio::sync::{OnceCell, RwLock, SetOnce};

use crate::{AppConfig, Binding, Bindings, Label};

#[derive(Clone)]
pub enum Location {
    Local,
    Remote(String),
    Unreachable,
}

tokio::task_local! {
    pub static CURRENT_LABEL: Label;
}

pub fn current_label() -> Label {
    CURRENT_LABEL.get()
}

pub fn scope_label<T, F>(label: Label, future: F) -> impl Future<Output = T>
where
    F: Future<Output = T>,
{
    CURRENT_LABEL.scope(label, future)
}

static RUNTIME: OnceCell<Runtime> = OnceCell::const_new();

struct Runtime {
    config: AppConfig,
    bindings: HashMap<Label, Binding>,
    local: RwLock<HashMap<Label, SetOnce<Arc<dyn Any + Send + Sync>>>>,
}

pub fn init(config: AppConfig, bindings: Bindings) {
    let rt = Runtime {
        config,
        bindings: bindings.comps,
        local: RwLock::new(HashMap::new()),
    };
    RUNTIME
        .set(rt)
        .ok()
        .expect("could not set RUNTIME: already set?");
}

fn runtime() -> &'static Runtime {
    RUNTIME.get().expect("RUNTIME not set")
}

pub fn config() -> &'static AppConfig {
    &runtime().config
}

pub fn binding() -> Binding {
    runtime()
        .bindings
        .get(current_label())
        .expect("no bindings for current job")
        .clone()
}

pub fn locate(target: Label) -> Location {
    let rt = runtime();
    if rt.config.placement(target) == rt.config.placement(current_label()) {
        return Location::Local;
    }
    match rt.bindings.get(target).unwrap() {
        Binding::None => Location::Unreachable,
        Binding::Http(_, url) => Location::Remote(url.clone()),
    }
}

pub async fn bind_local<T: Send + Sync + 'static>(bind: T) {
    runtime()
        .local
        .write()
        .await
        .entry(current_label())
        .or_insert(SetOnce::new())
        .set(Arc::new(bind))
        .unwrap();
}

pub async fn connect_local<T: Send + Sync + 'static>(target: Label) -> Arc<T> {
    let _ = {
        runtime()
            .local
            .write()
            .await
            .entry(target)
            .or_insert(SetOnce::new());
    };
    runtime()
        .local
        .read()
        .await
        .get(target)
        .unwrap()
        .wait()
        .await
        .clone()
        .downcast()
        .unwrap()
}

/*

static LOCAL_BINDINGS: LazyLock<RwLock<LocalBindings>> =
    LazyLock::new(|| RwLock::new(LocalBindings::new()));

struct LocalBindings {
    bindings: HashMap<Label, OnceLock<LocalBindingDynamic>>,
}

struct LocalBindingDynamic {
    label: Label,
    handler: Arc<dyn Any + Send + Sync + 'static>,
}

pub struct LocalBinding<T> {
    label: Label,
    handler: Arc<T>,
}

impl LocalBindings {
    fn new() -> LocalBindings {
        LocalBindings {
            bindings: HashMap::new(),
        }
    }
}

pub fn bind_local<T: Any + Send + Sync + 'static>(value: T) -> Result<(), &'static str> {
    let label = current_label().ok_or("no current label")?;

    let binding = LocalBindingDynamic {
        label,
        handler: Arc::new(value),
    };

    LOCAL_BINDINGS
        .read()
        .or(Err("could not lock LOCAL_BINDINGS: rwlock is poisoned"))?
        .bindings
        .get(label)
        .ok_or("internal error: slot not initialized for current label")?
        .set(binding)
        .or(Err("slot was already initialized"))?;

    Ok(())
}

fn connect_local<T>(target_label: Label) -> Result<LocalBinding<T>, &'static str> {
    let target: &LocalBindingDynamic = LOCAL_BINDINGS
        .read()
        .or(Err("could not lock LOCAL_BINDINGS: rwlock is poisoned"))?
        .bindings
        .get(target_label)
        .ok_or("no slot for target label. is target local?")?
        .wait();
}
*/
