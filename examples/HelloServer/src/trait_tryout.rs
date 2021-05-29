use bincode::serialize;
use serde::{Serialize, Deserialize};
use someip::{FieldError, config::Configuration, server::Server, tasks::ConnectionInfo, SomeIpHeader};
use crate::{connection::SomeIPCodec, someip_codec::SomeIpPacket};
use bytes::{Bytes, BytesMut};
use futures::SinkExt;
use std::{fmt::Write, iter::FromIterator, net::SocketAddr, str, sync::{Arc, Mutex}, time::Duration};
use tokio::runtime::Runtime;

use self::interface::HelloWorldServer;


mod interface {
    use generator_macros::*;
    use std::io;
    use serde::{Serialize, Deserialize};
    use thiserror::Error;




    #[derive(Serialize,Deserialize)]
    pub struct Field1 {
        a: u32,
        b: u16,
        c: String,
    }

    #[derive(Serialize,Deserialize)]
    pub struct Event1 {
        a: u32,
        b: String,
        c: String,
    }

    #[derive(Error, Debug,Serialize, Deserialize)]
    pub enum HelloWorldError {
        #[error("Foo Error")]
        FooError(i32),
        #[error("Bar error")]
        BarError(String),
    }

    #[service(fields([1]value1:Field1,[2]value2:String, [3]value3: u32),
        events([1 =>10]value1:Event1, [2=>10]value2:String, [3=>10]value3: u32), 
        method_ids([1]echo_int, [2]echo_string))]
    pub trait HelloWorld {
        fn echo_int(&self, value: i32, value1: Field1) -> Result<i32, HelloWorldError>;
        fn echo_string(&self, value: String) -> Result<String, HelloWorldError>;
    }

    pub struct HelloWorldServer {

    }
}



impl Default for interface::HelloWorldServer {
    fn default() -> Self {
        interface::HelloWorldServer {

        }
    }
}

impl someip::server::ServerRequestHandler for HelloWorldServer {
    fn handle(&self, message: SomeIpPacket) -> Option<SomeIpPacket> {
        interface::dispatcher::dispatch(self, message)
    }
}

impl interface::HelloWorld for interface::HelloWorldServer {
    fn echo_int(&self, value: i32, value1: interface::Field1) -> Result<i32, interface::HelloWorldError> {
        Ok(value)
    }

    fn echo_string(&self, value: String) -> Result<String, interface::HelloWorldError> {
        Ok(value)
    }

    fn set_value1(& self, _: interface::Field1) -> Result<(), FieldError> { Ok(()) }

    fn get_value1(&self) -> Result<&interface::Field1, FieldError> { todo!() }
    fn set_value2(&self, _: std::string::String) -> Result<(), FieldError> { Ok(())}
    fn get_value2(&self) -> Result<&std::string::String, FieldError> { todo!() }
    fn set_value3(&self, _: u32) -> Result<(), FieldError> { Ok(()) }
    fn get_value3(&self) -> Result<&u32, FieldError> { todo!() }

}

pub fn start_server() {


    let rt = Runtime::new().unwrap();
    let config = Configuration::default();

    let at = "127.0.0.1:8090".parse::<SocketAddr>().unwrap();
    println!("Test");
    let _result = rt.block_on(async {
        let (tx, mut rx) = Server::create_notify_channel(1);

        tokio::spawn(async move {
            loop {
                if let Some(msg) = rx.recv().await {
                    match msg {
                        ConnectionInfo::NewTcpConnection((sender, i)) => {
                            println!("New connection from {}", i);
                        }
                        ConnectionInfo::ConnectionDropped(_i) => {}
                        ConnectionInfo::NewUdpConnection((sender, i)) => {
                            println!("Udp connection established");
                        }
                    }
                }
            }
        });

        tokio::spawn(async move {
            let test_service = Box::new(interface::HelloWorldServer::default());
            let service = Arc::new(Mutex::new(test_service));
            println!("Going to run server");
            let res = Server::serve(at, service, config, 45, tx).await;
            println!("Server terminated");
            if let Err(e) = res {
                println!("Server error:{}", e);
            }
        });

        tokio::time::sleep(Duration::from_millis(20)).await;

        let addr = "127.0.0.1:8090".parse::<SocketAddr>().unwrap();
        let mut tx_connection = SomeIPCodec::default().connect(&addr).await.unwrap();

        let mut header = SomeIpHeader::default();
       header.set_service_id(45);
       header.set_method_id(0x01);
       let val:i32 = 42;
       let reply_raw = serialize(& val).unwrap();

       let payload = Bytes::from(reply_raw);
        let packet = SomeIpPacket::new(header, payload);

       tx_connection.send(packet).await;

        println!("Sending terminate");
        tokio::time::sleep(Duration::from_millis(2000)).await;
        //let res = &mut handle.terminate().await;
    });
}