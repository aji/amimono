use std::{any::Any, collections::HashMap, marker::PhantomData, sync::Arc};

use futures::{future::BoxFuture, lock::Mutex};
use log::info;
use tokio::sync::RwLock;

use crate::{AppConfig, Label};

#[derive(Clone, Debug)]
enum RuntimeScope {
    Job(Label),
    Component(Label),
}

#[derive(Clone)]
pub struct Runtime {
    scope: RuntimeScope,
    data: Arc<RuntimeData>,
}

struct RuntimeData {
    local: RwLock<HashMap<Label, Box<dyn LocalHandler>>>,
}

impl Runtime {
    pub fn new(job: Label, cf: &AppConfig) -> Runtime {
        for comp in cf.job(job).components() {}
        Runtime {
            scope: RuntimeScope::Job(job),
            data: Arc::new(RuntimeData {
                local: RwLock::new(HashMap::new()),
            }),
        }
    }

    pub fn for_component(&self, comp: Label) -> Runtime {
        let next = match self.scope {
            RuntimeScope::Job(_) => RuntimeScope::Component(comp),
            RuntimeScope::Component(_) => panic!(),
        };
        Runtime {
            scope: next,
            data: self.data.clone(),
        }
    }

    pub async fn bind_local<H, Q, A>(&self, handler: H)
    where
        H: AsyncFn(Q) -> A + Send + Sync + 'static,
        for<'a> H::CallRefFuture<'a>: Send,
        Q: Sized + Send + Sync + Any,
        A: Sized + Send + Sync + Any,
    {
        let label = match self.scope {
            RuntimeScope::Job(_) => panic!(),
            RuntimeScope::Component(x) => x,
        };
        info!("bind_local: {}", label);
        self.data.local.write().await.insert(
            label,
            Box::new(ErasedLocalHandler {
                inner: handler,
                x: PhantomData,
            }),
        );
    }

    pub async fn call_local<Q, A>(&self, target: Label, q: Q) -> A
    where
        Q: Sized + Send + Sync + Any,
        A: Sized + Send + Sync + Any,
    {
        info!("call_local: {:?} -> {}", self.scope, target);
        let local = self.data.local.read().await;
        let handler = local.get(target).unwrap();
        let a = handler.handle(Box::new(q)).await;
        *a.downcast::<A>().unwrap()
    }
}

trait LocalHandler: Send + Sync {
    fn handle(&self, q: Box<dyn Any + Send>) -> BoxFuture<Box<dyn Any + Send>>;
}

struct ErasedLocalHandler<H, Q, A> {
    inner: H,
    x: PhantomData<(Q, A)>,
}
impl<H, Q, A> LocalHandler for ErasedLocalHandler<H, Q, A>
where
    H: AsyncFn(Q) -> A + Send + Sync,
    for<'a> H::CallRefFuture<'a>: Send,
    Q: Sized + Send + Sync + Any,
    A: Sized + Send + Sync + Any,
{
    fn handle(&self, q: Box<dyn Any + Send>) -> BoxFuture<Box<dyn Any + Send>> {
        Box::pin(async move {
            let a: A = (self.inner)(*q.downcast::<Q>().unwrap()).await;
            Box::new(a) as Box<dyn Any + Send>
        })
    }
}
