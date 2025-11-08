use crate::{Component, LocalBinding, RemoteBinding};

pub trait Context {
    fn binding(&self) -> &LocalBinding;
    fn locate<C: Component>(&self) -> &RemoteBinding;
}
