use futures::future::BoxFuture;

use crate::{BindingType, Label, Runtime, toml::ComponentToml};

pub trait ComponentMain: Send + Sync + 'static {
    fn main_async(&self, rt: Runtime) -> BoxFuture<()>;
}

pub struct Component {
    label: Label,
    binding: BindingType,
    main: Box<dyn ComponentMain>,
}

impl Component {
    pub fn new<C: ComponentMain>(label: Label, binding: BindingType, main: C) -> Component {
        Component {
            label,
            binding,
            main: Box::new(main),
        }
    }

    pub fn from_async_fn<F>(label: Label, binding: BindingType, main: F) -> Component
    where
        F: AsyncFn(Runtime) -> () + Send + Sync + 'static,
        for<'a> F::CallRefFuture<'a>: Send,
    {
        Component::new(label, binding, AsyncComponentMain(main))
    }

    pub fn label(&self) -> Label {
        self.label
    }

    pub fn binding(&self) -> BindingType {
        self.binding
    }

    pub fn to_toml(&self) -> ComponentToml {
        ComponentToml {
            binding: self.binding.to_toml(),
        }
    }

    pub fn start(&self, rt: Runtime) -> BoxFuture<()> {
        self.main.main_async(rt)
    }
}

struct AsyncComponentMain<F>(F);

impl<F> ComponentMain for AsyncComponentMain<F>
where
    F: AsyncFn(Runtime) -> () + Send + Sync + 'static,
    for<'a> F::CallRefFuture<'a>: Send,
{
    fn main_async(&self, rt: Runtime) -> BoxFuture<()> {
        Box::pin((self.0)(rt))
    }
}
