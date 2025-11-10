use crate::{Component, Label, Runtime, component::ComponentMain};

struct AsyncComponentFn<F>(F);

impl<F> ComponentMain for AsyncComponentFn<F>
where
    F: AsyncFn(Runtime),
{
    fn main(&self, rt: Runtime) {
        let job = (self.0)(rt);
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(job)
    }
}

pub fn async_component_fn<F>(label: Label, func: F) -> Component
where
    F: AsyncFn(Runtime) + 'static,
{
    Component::new(label, Box::new(AsyncComponentFn(func)))
}
