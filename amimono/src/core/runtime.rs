use std::{any::Any, collections::HashMap, sync::Arc};

use futures::future::LocalBoxFuture;
use tokio::sync::SetOnce;

use crate::{AppConfig, Label};

#[derive(Copy, Clone, Debug)]
enum RuntimeScope {
    Global,
    Local(Label),
}

#[derive(Clone)]
pub enum Location {
    Local,
    Remote(String),
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
    local: HashMap<Label, SetOnce<LocalBinding>>,
}

impl Runtime {
    pub(crate) fn new(cf: &AppConfig, job: Label) -> Runtime {
        let mut data = RuntimeData {
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

    fn relocated(&self, target: Label) -> Runtime {
        Runtime {
            scope: RuntimeScope::Local(target),
            data: self.data.clone(),
        }
    }

    pub(crate) fn place(&self, target: Label) -> Runtime {
        if let RuntimeScope::Global = self.scope {
            self.relocated(target)
        } else {
            panic!("cannot place runtime already scoped to {:?}", self.scope);
        }
    }

    pub(crate) fn locate(&self, _target: Label) -> Location {
        Location::Local
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
        Q: 'static,
        A: 'static,
    {
        let binding = match self.data.local.get(target) {
            Some(x) => x.wait().await,
            None => panic!("no local binding for {:?}", target),
        };
        binding.call(self.relocated(target), q).await
    }
}

pub(crate) trait LocalBindingHandler<Q, A> {
    fn call(&self, rt: Runtime, q: Q) -> LocalBoxFuture<A>;
}

impl<F> LocalBindingHandler<Box<dyn Any>, Box<dyn Any>> for F
where
    F: AsyncFn(Runtime, Box<dyn Any>) -> Box<dyn Any>,
{
    fn call(&self, rt: Runtime, q: Box<dyn Any>) -> LocalBoxFuture<Box<dyn Any>> {
        Box::pin((*self)(rt, q))
    }
}

pub(crate) enum LocalBinding {
    Dynamic(Box<dyn LocalBindingHandler<Box<dyn Any>, Box<dyn Any>>>),
}

impl LocalBinding {
    pub fn new<Q, A, F>(handler: F) -> LocalBinding
    where
        F: LocalBindingHandler<Q, A> + 'static,
        Q: 'static,
        A: 'static,
    {
        let outer = async move |rt, q_box: Box<dyn Any>| {
            let q: Q = *q_box.downcast().unwrap();
            let a: A = handler.call(rt, q).await;
            Box::new(a) as Box<dyn Any>
        };
        LocalBinding::Dynamic(Box::new(outer))
    }

    async fn call<Q, A>(&self, rt: Runtime, q: Q) -> A
    where
        Q: 'static,
        A: 'static,
    {
        let LocalBinding::Dynamic(handler) = self;
        let a_box = handler.call(rt, Box::new(q)).await;
        *a_box.downcast().unwrap()
    }
}
