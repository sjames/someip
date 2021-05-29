use serde::{Deserialize, Serialize};
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FieldError {
    #[error("Protocol Error")]
    IoError(#[from] io::Error),
    #[error("unknown error")]
    Unknown,
}

#[derive(Error, Debug, Deserialize, Serialize)]
pub enum Error {
    #[error("unknown error")]
    Unknown,
}
