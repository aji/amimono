use crate::{BindingType, Configuration, Context};

pub trait Component: Sized + 'static {
    const LABEL: &'static str;

    const BINDING: BindingType = BindingType::None;

    fn main<X: Context>(ctx: &X);

    fn place<X: Configuration>(cf: &mut X) {
        cf.place::<Self>();
    }
}
