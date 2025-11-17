use futures::future::BoxFuture;

use crate::{BindingType, Label, toml::ComponentToml};

pub trait ComponentMain: Send + Sync + 'static {
    fn main_async(&'_ self) -> BoxFuture<'_, ()>;
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

    pub fn from_async_fn<F, Fut>(label: Label, binding: BindingType, main: F) -> Component
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        for<'a> Fut: Future<Output = ()> + Send + 'a,
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

    pub fn start(&'_ self) -> BoxFuture<'_, ()> {
        self.main.main_async()
    }
}

struct AsyncComponentMain<F>(F);

impl<F, Fut> ComponentMain for AsyncComponentMain<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    for<'a> Fut: Future<Output = ()> + Send + 'a,
{
    fn main_async(&'_ self) -> BoxFuture<'_, ()> {
        Box::pin((self.0)())
    }
}
