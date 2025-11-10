use futures::future::BoxFuture;

use crate::{Label, Runtime};

pub struct Component {
    label: Label,
    main: Box<dyn ComponentMain>,
}

impl Component {
    pub(crate) fn new(label: Label, main: Box<dyn ComponentMain>) -> Component {
        Component { label, main }
    }
    pub fn label(&self) -> Label {
        self.label
    }
    pub fn main_blocking(&self, rt: Runtime) {
        self.main.main_blocking(rt)
    }
    pub fn main_async(&self, rt: Runtime) -> impl Future<Output = ()> {
        self.main.main_async(rt)
    }
}

pub(crate) trait ComponentMain: Send + Sync {
    fn main_blocking(&self, rt: Runtime);
    fn main_async(&self, rt: Runtime) -> BoxFuture<()>;
}
