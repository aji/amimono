use futures::future::LocalBoxFuture;

use crate::{Label, Runtime};

pub trait ComponentMain: Send + Sync + 'static {
    fn main_async(&self, rt: Runtime) -> LocalBoxFuture<()>;
}

pub struct Component {
    label: Label,
    main: Box<dyn ComponentMain>,
}

impl Component {
    pub fn new<C: ComponentMain>(label: Label, main: C) -> Component {
        Component {
            label,
            main: Box::new(main),
        }
    }

    pub fn from_async_fn<F>(label: Label, main: F) -> Component
    where
        F: AsyncFn(Runtime) -> () + Send + Sync + 'static,
    {
        Component::new(label, AsyncComponentMain(main))
    }

    pub fn label(&self) -> Label {
        self.label
    }

    pub fn start(&self, rt: Runtime) -> LocalBoxFuture<()> {
        self.main.main_async(rt)
    }
}

struct AsyncComponentMain<F>(F);

impl<F> ComponentMain for AsyncComponentMain<F>
where
    F: AsyncFn(Runtime) -> () + Send + Sync + 'static,
{
    fn main_async(&self, rt: Runtime) -> LocalBoxFuture<()> {
        Box::pin((self.0)(rt))
    }
}
