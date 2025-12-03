use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Internal(String),
    Other(Box<dyn std::error::Error>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Internal(s) => write!(f, "internal error: {s}"),
            Error::Other(e) => write!(f, "other: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Error::Internal(value)
    }
}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Error::Internal(value.to_owned())
    }
}
