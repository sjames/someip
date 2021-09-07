/*
    Copyright 2021 Sojan James
    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at
        http://www.apache.org/licenses/LICENSE-2.0
    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.
*/

use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FieldError {
    #[error("Protocol Error")]
    IoError(#[from] io::Error),
    #[error("unknown error")]
    Unknown,
}

#[derive(Error, Debug)]
pub enum MethodError<T>
where
    T: std::fmt::Debug + std::error::Error + serde::Serialize + serde::de::DeserializeOwned,
{
    #[error("Method error")]
    Error(T),
    #[error("Connection error")]
    ConnectionError,
    #[error("Response payload was invalid")]
    InvalidResponsePayload,
    #[error("Error payload was invalid")]
    InvalidErrorPayload,
}

impl<T> MethodError<T>
where
    T: std::fmt::Debug + std::error::Error + serde::Serialize + serde::de::DeserializeOwned,
{
    pub fn is_service_error(&self) -> bool {
        matches!(self, Self::Error(_e))
    }

    pub fn into_service_error(self) -> Option<T> {
        match self {
            Self::Error(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Error, Debug)]
pub enum PropertyError {
    #[error("Connection error")]
    ConnectionError,
    #[error("Response payload was invalid")]
    InvalidResponsePayload,
    #[error("Call was cancelled")]
    Cancelled,
}

/// Generic errors
#[derive(Error, Debug)]
pub enum Error {
    #[error("Connection error")]
    ConnectionError,
    #[error("IO Error")]
    IoError(#[from] io::Error),
}
