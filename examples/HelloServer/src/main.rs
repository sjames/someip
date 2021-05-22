use someip::*;

use std::io::{self};
use bytes::Bytes;

use bincode::{deserialize, serialize};
use serde::*;

mod trait_tryout;

struct Field<T>(T);

impl<T> Field<T> {
    pub fn set(&mut self, val: T) {
        self.0 = val
    }
    pub fn get(&self) -> &T {
        &self.0
    }
}

pub trait HelloServer {

    fn get_field1() -> Result<u32,io::Error>;
    fn set_field1() -> Result<(),io::Error>;

    fn event() -> Result<(),io::Error> {
        todo!();
    }

    fn echo_int(&mut self, param: EchoIntCallParams) -> Result<EchoIntResponseParams, io::Error>;

    fn echo_string(
        &mut self,
        param: EchoStringCallParams,
    ) -> Result<EchoStringResponseParams, io::Error>;
}



struct HelloServerImpl {
    field_u32: Field<u32>,
    field_string: Field<String>,
}

#[derive(Debug,Serialize,Deserialize)]
enum HelloServerEvent {
    Event1(u32),
    Event2(String),
}

#[derive(Debug,Serialize,Deserialize)]
pub struct EchoIntCallParams {
    pub value: i32,
}


#[derive(Debug,Serialize,Deserialize)]
pub struct EchoIntResponseParams {
    pub value: i32,
}
#[derive(Debug,Serialize,Deserialize)]
pub struct EchoStringCallParams {
    pub value: String,
}
#[derive(Debug,Serialize,Deserialize)]
pub struct EchoStringResponseParams {
    pub value: String,
}

impl HelloServerImpl {
    fn echo_int(&mut self, param: EchoIntCallParams) -> Result<EchoIntResponseParams, io::Error> {
        Ok(EchoIntResponseParams { value: param.value })
    }

    fn echo_string(
        &mut self,
        param: EchoStringCallParams,
    ) -> Result<EchoStringResponseParams, io::Error> {
        Ok(EchoStringResponseParams { value: param.value })
    }

    pub fn send_event(event: HelloServerEvent) -> Result<(), io::Error> {
        Ok(())
    }
}

/// This function should be auto-generated
impl someip::server::ServerRequestHandler for HelloServerImpl {
    fn handle(&mut self, pkt: SomeIpPacket) -> Option<SomeIpPacket> {
        match pkt.header().event_or_method_id() {
            // MethodID 0 -> echo_int
            0 => {
                let params_raw = pkt.payload().as_ref();
                let param : EchoIntCallParams = deserialize(params_raw).unwrap();
                let res = self.echo_int(param);
                match res {
                    Ok(r) => {
                        let reply_raw = serialize(&r).unwrap();
                        let reply_payload = Bytes::from(reply_raw);
                        Some(SomeIpPacket::reply_packet_from(pkt, someip_codec::ReturnCode::Ok, reply_payload))                      
                    }
                    Err(e) => {
                       // let reply_raw = serialize(&e).unwrap();
                       // let reply_payload = Bytes::from(reply_raw);
                        Some(SomeIpPacket::error_packet_from(pkt, someip_codec::ReturnCode::NotOk, Bytes::new()))  
                    }
                }
            }   
            1 => {
                todo!()
            } 
            // Events
            0x8000 => {
                todo!()
            }
            _ => {
                Some(SomeIpPacket::error_packet_from(pkt, someip_codec::ReturnCode::UnknownMethod, Bytes::new()))
            }
        }
    }
}

fn main() {
    println!("Hello, world!");
}
