#![feature(async_fn_traits)]

pub(crate) mod r#async;
pub(crate) mod binding;
pub(crate) mod component;
pub(crate) mod config;
pub(crate) mod job;
pub(crate) mod local;
pub(crate) mod run;
pub(crate) mod runtime;

pub type Label = &'static str;

pub use r#async::async_component_fn;
pub use component::Component;
pub use config::{AppBuilder, AppConfig, JobBuilder, JobConfig};
pub use run::run;
pub use runtime::Runtime;
