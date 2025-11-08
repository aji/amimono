use crate::{Component, LocalBinding, RemoteBinding};

pub trait Context: Send + Sync + 'static {
    fn binding(&self) -> &LocalBinding;
    fn locate<C: Component>(&self) -> &RemoteBinding;
}
