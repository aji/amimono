use crate::Configuration;

pub trait Application: 'static {
    const LABEL: &'static str;

    fn setup<X: Configuration>(&self, cf: &mut X);
}
