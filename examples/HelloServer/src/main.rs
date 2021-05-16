use someip::*;
use std::io;

struct Field<T>(T);

impl<T> Field<T> {
    pub fn set(&mut self, val: T) {
        self.0 = val
    }
    pub fn get(&self) -> &T {
        &self.0
    }
}

struct HelloServer {
    field_u32: Field<u32>,
    field_string: Field<String>,
}

enum HelloServerEvent {
    Event1(u32),
    Event2(String),
}

impl HelloServer {
    fn echo_int(&mut self, value: i32) -> Result<i32, io::Error> {
        Ok(value)
    }

    fn echo_string(&mut self, value: &str) -> Result<String, io::Error> {
        Ok(value.to_string())
    }

    pub fn send_event(event: HelloServerEvent) -> Result<(), io::Error> {
        Ok(())
    }
}

impl someip::server::ServerRequestHandler for HelloServer {
    fn handle(&mut self, pkt: SomeIpPacket) -> Option<SomeIpPacket> {
        todo!()
    }
}

fn main() {
    println!("Hello, world!");
}
