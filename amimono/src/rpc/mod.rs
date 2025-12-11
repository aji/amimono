//! Subsystem for building components with an RPC interface.
//!
//! It's recommended to use this module via the `rpc_component!` macro When
//! using that macro, it's rarely necessary to use any of the definitions in
//! this module directly, however they are documented for the sake of
//! completeness.

mod client;
mod component;
mod http;
mod macros;

pub use client::RpcClient;
pub use component::{RpcComponent, RpcComponentKind, RpcMessage};
pub use http::PORT;

pub type RpcError = crate::AppError;
pub type RpcResult<T> = crate::AppResult<T>;
