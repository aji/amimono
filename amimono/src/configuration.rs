use crate::Component;

pub trait Configuration {
    fn place<C: Component>(&mut self);
}
