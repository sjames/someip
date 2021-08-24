//! A SOME/IP Implementation for Rust
//!
//! Caveats
//! * This implementation uses bincode serialization
//! * Service Discover is not yet implemented
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
pub use server::{Server, ServerRequestHandler};
pub use someip_codec::{MessageType, ReturnCode, SomeIpHeader, SomeIpPacket};
pub use tasks::ConnectionMessage;
use tasks::{ConnectionInfo, DispatcherCommand, DispatcherReply};
pub use {client::Client, client::ReplyData};
