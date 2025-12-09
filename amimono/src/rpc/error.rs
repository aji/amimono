use std::fmt;

use serde::{Deserialize, Serialize};

use crate::retry::RetryError;

pub type RpcResult<T> = std::result::Result<T, RpcError>;

/// An error when making an RPC call.
#[derive(Clone, Serialize, Deserialize)]
pub enum RpcError {
    /// A spurious error with an unstructured string message. These can
    /// generally be assumed to be recoverable.
    Spurious(String),

    /// A miscellaneous error with an unstructured string message. These should
    /// generally be assumed to be unrecoverable.
    Misc(String),

    /// An error together with a location. This variant is constructed
    /// automatically by `RpcClient` when making a call, and can be nested
    /// several layers deep. Use `root_cause` to get the innermost `RpcError`.
    Downstream(String, Box<RpcError>),
}

impl RpcError {
    /// Unwrap layers of caused-by nesting to get the innermost error.
    pub fn root_cause(&self) -> &RpcError {
        match self {
            RpcError::Downstream(_, e) => e.root_cause(),
            _ => self,
        }
    }
}

impl RetryError for RpcError {
    fn should_retry(&self) -> bool {
        match self {
            RpcError::Spurious(_) => true,
            RpcError::Misc(_) => false,
            RpcError::Downstream(_, e) => e.should_retry(),
        }
    }
}

impl axum::response::IntoResponse for RpcError {
    fn into_response(self) -> axum::response::Response {
        let res = (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(self),
        );
        res.into_response()
    }
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcError::Spurious(s) => write!(f, "spurious: {s}"),
            RpcError::Misc(s) => write!(f, "rpc error: {s}"),
            RpcError::Downstream(at, e) => write!(f, "{at}: {e}"),
        }
    }
}

impl From<String> for RpcError {
    fn from(s: String) -> Self {
        RpcError::Misc(s)
    }
}

impl From<&str> for RpcError {
    fn from(value: &str) -> Self {
        RpcError::Misc(value.to_owned())
    }
}

impl From<crate::error::Error> for RpcError {
    fn from(value: crate::error::Error) -> Self {
        RpcError::Misc(format!("amimono error: {value}"))
    }
}

impl From<reqwest::Error> for RpcError {
    fn from(value: reqwest::Error) -> Self {
        if value.is_timeout() {
            let origin = match value.url() {
                Some(u) => u.origin().ascii_serialization(),
                None => "(unknown)".to_owned(),
            };
            RpcError::Spurious(format!("http timeout at {origin}"))
        } else {
            RpcError::Misc(format!("http error: {value}"))
        }
    }
}

impl From<serde_json::Error> for RpcError {
    fn from(value: serde_json::Error) -> Self {
        RpcError::Misc(format!("json error: {value}"))
    }
}

impl From<std::io::Error> for RpcError {
    fn from(value: std::io::Error) -> Self {
        RpcError::Misc(format!("io error: {value}"))
    }
}

impl From<tokio::task::JoinError> for RpcError {
    fn from(value: tokio::task::JoinError) -> Self {
        match value.try_into_panic() {
            Ok(e) => std::panic::resume_unwind(e),
            Err(e) => match e.is_cancelled() {
                true => RpcError::Misc(format!("task cancelled")),
                false => RpcError::Misc(format!("tokio join error")),
            },
        }
    }
}
