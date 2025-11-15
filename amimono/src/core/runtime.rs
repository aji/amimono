use std::{any::Any, collections::HashMap, sync::Arc};

use tokio::sync::SetOnce;

use crate::{AppConfig, Binding, Bindings, Label};

#[derive(Copy, Clone, Debug)]
enum RuntimeScope {
    Global,
    Local(Label),
}

#[derive(Clone)]
pub enum Location {
    Local,
    Remote(String),
    Unreachable,
}

impl RuntimeScope {
    fn local(self) -> Option<Label> {
        match self {
            RuntimeScope::Global => None,
            RuntimeScope::Local(x) => Some(x),
        }
    }
}

#[derive(Clone)]
pub struct Runtime {
    scope: RuntimeScope,
    data: Arc<RuntimeData>,
}

struct RuntimeData {
    bindings: HashMap<Label, Binding>,
    local: HashMap<Label, SetOnce<Arc<dyn Any + Send + Sync>>>,
}

impl Runtime {
    pub(crate) fn new(cf: &AppConfig, bindings: &Bindings, job: &str) -> Runtime {
        let mut data = RuntimeData {
            bindings: bindings.comps.clone(),
            local: HashMap::new(),
        };
        for comp in cf.job(job).components() {
            data.local.insert(comp.label(), SetOnce::new());
        }
        Runtime {
            scope: RuntimeScope::Global,
            data: Arc::new(data),
        }
    }

    pub fn relocated(&self, target: Label) -> Runtime {
        Runtime {
            scope: RuntimeScope::Local(target),
            data: self.data.clone(),
        }
    }

    pub(crate) fn place(&self, target: Label) -> Runtime {
        match self.scope {
            RuntimeScope::Global => self.relocated(target),
            RuntimeScope::Local(_) => {
                panic!("cannot place runtime already scoped to {:?}", self.scope)
            }
        }
    }

    pub fn binding(&self) -> &Binding {
        match self.scope {
            RuntimeScope::Global => panic!("global scope does not have bindings"),
            RuntimeScope::Local(x) => self.data.bindings.get(x).unwrap(),
        }
    }

    pub(crate) fn locate(&self, target: Label) -> Location {
        if self.data.local.contains_key(target) {
            return Location::Local;
        }
        match self.data.bindings.get(target).unwrap() {
            Binding::None => Location::Unreachable,
            Binding::Http(_, url) => Location::Remote(url.clone()),
        }
    }

    pub(crate) fn bind_local<B: Send + Sync + 'static>(&self, binding: B) {
        let label = self
            .scope
            .local()
            .expect("cannot call bind_local in non-local scope");
        log::debug!("{} setting local binding", label);
        let res = match self.data.local.get(label) {
            Some(x) => x.set(Arc::new(binding)),
            None => panic!("no local binding for {:?}", label),
        };
        if let Err(_) = res {
            panic!("failed to set local binding {:?}", label);
        }
    }

    pub(crate) async fn connect_local<B: Send + Sync + 'static>(&self, target: Label) -> Arc<B> {
        // TODO: this is susceptible to deadlocks if components connect before
        // binding locally and there is a dependency cycle
        let scope = match self.scope {
            RuntimeScope::Global => "(global)",
            RuntimeScope::Local(x) => x,
        };
        log::debug!("{} connecting to {} via local binding", scope, target);
        match self.data.local.get(target) {
            Some(x) => x.wait().await.clone().downcast().unwrap(),
            None => panic!("no local binding for {:?}", target),
        }
    }
}
