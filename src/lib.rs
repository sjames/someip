pub mod client;
pub mod config;
pub mod connection;
pub mod error;
pub mod field;
pub mod server;
pub mod someip_codec;
pub mod tasks;

pub use config::Configuration;
pub use error::{FieldError, MethodError};
pub use someip_codec::{MessageType, SomeIpHeader, SomeIpPacket};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
