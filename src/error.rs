use core::fmt;
use std::sync::PoisonError;

use tokio::time::error::Elapsed;

#[derive(Debug)]
pub enum Error {
    Client(String),
    Ble(btleplug::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Client(value) => write!(fmt, "{}", value),
            Error::Ble(value) => write!(fmt, "Bluetooth platform error: {}", value),
        }
    }
}

impl Error {
    pub fn client(msg: impl Into<String>) -> Self {
        Self::Client(msg.into())
    }
}

impl From<btleplug::Error> for Error {
    fn from(e: btleplug::Error) -> Self {
        Self::Ble(e)
    }
}

impl From<&str> for Error {
    fn from(e: &str) -> Self {
        Self::client(e.to_owned())
    }
}

impl From<tokio::time::error::Error> for Error {
    fn from(e: tokio::time::error::Error) -> Self {
        Self::client(e.to_string())
    }
}

impl From<Elapsed> for Error {
    fn from(_e: Elapsed) -> Self {
        Self::client("Timeout error")
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(_e: PoisonError<T>) -> Self {
        Self::from("Asynchronous error. Can not recursively lock mutex.")
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::client(e.to_string())
    }
}
