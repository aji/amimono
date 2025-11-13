use std::{any::Any, collections::HashMap, sync::Arc};

use futures::future::BoxFuture;
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
    bindings: Arc<Bindings>,
    data: Arc<RuntimeData>,
}

struct RuntimeData {
    local: HashMap<Label, SetOnce<LocalBinding>>,
}

impl Runtime {
    pub(crate) fn new(cf: &AppConfig, bindings: Arc<Bindings>, job: &str) -> Runtime {
        let mut data = RuntimeData {
            local: HashMap::new(),
        };
        for comp in cf.job(job).components() {
            data.local.insert(comp.label(), SetOnce::new());
        }
        Runtime {
            scope: RuntimeScope::Global,
            bindings,
            data: Arc::new(data),
        }
    }

    fn relocated(&self, target: Label) -> Runtime {
        Runtime {
            scope: RuntimeScope::Local(target),
            bindings: self.bindings.clone(),
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
            RuntimeScope::Local(x) => self.bindings.get(x),
        }
    }

    pub(crate) fn locate(&self, target: Label) -> Location {
        if self.data.local.contains_key(target) {
            return Location::Local;
        }
        match self.bindings.get(target) {
            Binding::None => Location::Unreachable,
            Binding::Http(_, url) => Location::Remote(url.clone()),
        }
    }

    pub(crate) fn bind_local(&self, binding: LocalBinding) {
        let label = self
            .scope
            .local()
            .expect("cannot call bind_local in non-local scope");
        let res = match self.data.local.get(label) {
            Some(x) => x.set(binding),
            None => panic!("no local binding for {:?}", label),
        };
        if let Err(_) = res {
            panic!("failed to set local binding {:?}", label);
        }
    }

    pub(crate) async fn call_local<Q, A>(&self, target: Label, q: Q) -> A
    where
        Q: Send + 'static,
        A: Send + 'static,
    {
        let binding = match self.data.local.get(target) {
            Some(x) => x.wait().await,
            None => panic!("no local binding for {:?}", target),
        };
        binding.call(self.relocated(target), q).await
    }
}

pub(crate) trait LocalBindingHandler<Q, A>: Send + Sync {
    fn call(&self, rt: Runtime, q: Q) -> BoxFuture<A>;
}

type Dynamic = Box<dyn Any + Send>;

impl<F> LocalBindingHandler<Dynamic, Dynamic> for F
where
    F: AsyncFn(Runtime, Dynamic) -> Dynamic + Send + Sync,
    for<'a> F::CallRefFuture<'a>: Send,
{
    fn call(&self, rt: Runtime, q: Dynamic) -> BoxFuture<Dynamic> {
        Box::pin((*self)(rt, q))
    }
}

pub(crate) enum LocalBinding {
    Dynamic(Box<dyn LocalBindingHandler<Dynamic, Dynamic>>),
}

impl LocalBinding {
    pub fn new<Q, A, F>(handler: F) -> LocalBinding
    where
        F: LocalBindingHandler<Q, A> + 'static,
        Q: Send + 'static,
        A: Send + 'static,
    {
        let outer = async move |rt, q_box: Dynamic| {
            let q: Q = *q_box.downcast().unwrap();
            let a: A = handler.call(rt, q).await;
            Box::new(a) as Dynamic
        };
        LocalBinding::Dynamic(Box::new(outer))
    }

    async fn call<Q, A>(&self, rt: Runtime, q: Q) -> A
    where
        Q: Send + 'static,
        A: Send + 'static,
    {
        let LocalBinding::Dynamic(handler) = self;
        let a_box = handler.call(rt, Box::new(q)).await;
        *a_box.downcast().unwrap()
    }
}
