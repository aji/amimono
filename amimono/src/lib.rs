extern crate rand;
extern crate serde;
extern crate serde_json;

pub mod local;
pub mod location;
pub mod traits;

pub use location::Location;
pub use traits::*;
