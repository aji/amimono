use crate::{BindingType, Context};

pub trait Component {
    const LABEL: &'static str;

    const BINDING: BindingType;

    fn main<X: Context>(ctx: &X);
}
