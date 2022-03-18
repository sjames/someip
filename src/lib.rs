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

//! A SOME/IP Implementation for Rust
//!
//! Caveats
//! * This implementation uses bincode serialization
//! * Service Discovery is not yet implemented
pub mod call_properties;
pub mod client;
pub mod config;
pub mod connection;
pub mod error;
pub mod field;
pub mod sd_messages;
pub mod sd_server;
pub mod sd_server_sm;
pub mod server;
pub mod someip_codec;
pub mod tasks;

#[cfg(test)]
mod tests;

// We reexport log and bincode for the generated code
pub use bincode;
pub use bytes;
pub use log;

pub use call_properties::CallProperties;
pub use config::Configuration;
pub use error::{FieldError, MethodError};
pub use field::Field;
pub use futures::future::BoxFuture;
pub use server::{CreateServerRequestHandler, Server, ServerRequestHandler, ServiceIdentifier};
pub use someip_codec::{MessageType, ReturnCode, SomeIpHeader, SomeIpPacket};
pub use tasks::ConnectionMessage;
use tasks::{ConnectionInfo, DispatcherCommand, DispatcherReply};
pub use {client::Client, client::ReplyData};
