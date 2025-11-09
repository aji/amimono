use crate::{Label, Runtime};

pub trait Component: 'static {
    fn label(&self) -> Label;

    fn main(&self, rt: Runtime);
}
