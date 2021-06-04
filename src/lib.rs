pub mod client;
pub mod config;
pub mod connection;
pub mod error;
pub mod field;
pub mod server;
pub mod service_discovery;
pub mod someip_codec;
pub mod tasks;

#[cfg(test)]
mod tests;

pub use config::Configuration;
pub use error::{FieldError, MethodError};
pub use field::Field;
pub use server::{Server, ServerRequestHandler};
pub use someip_codec::{MessageType, ReturnCode, SomeIpHeader, SomeIpPacket};
pub use tasks::{ConnectionInfo, ConnectionMessage, DispatcherCommand, DispatcherReply};
pub use {client::Client, client::ReplyData};
