pub mod client;
pub mod config;
pub mod connection;
pub mod server;
pub mod someip_codec;
pub mod tasks;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
