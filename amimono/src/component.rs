use crate::{Label, Runtime};

pub struct Component {
    label: Label,
    main: Box<dyn ComponentMain>,
}

impl Component {
    pub fn new(label: Label, main: Box<dyn ComponentMain>) -> Component {
        Component { label, main }
    }
    pub fn label(&self) -> Label {
        self.label
    }
    pub fn main(&self, rt: Runtime) {
        self.main.main(rt)
    }
}

pub(crate) trait ComponentMain {
    fn main(&self, rt: Runtime);
}
