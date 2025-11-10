use futures::future::BoxFuture;

use crate::{Component, Label, Runtime, component::ComponentMain};

struct AsyncComponentFn<F>(F);

impl<F> ComponentMain for AsyncComponentFn<F>
where
    F: AsyncFn(Runtime) + Send + Sync,
    for<'a> F::CallRefFuture<'a>: Send,
{
    fn main_blocking(&self, rt: Runtime) {
        let job = (self.0)(rt);
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(job)
    }

    fn main_async(&self, rt: Runtime) -> BoxFuture<()> {
        Box::pin((self.0)(rt))
    }
}

pub fn async_component_fn<F>(label: Label, func: F) -> Component
where
    F: AsyncFn(Runtime) + Send + Sync + 'static,
    for<'a> F::CallRefFuture<'a>: Send,
{
    Component::new(label, Box::new(AsyncComponentFn(func)))
}
