use bincode::serialize;
use serde::{Serialize, Deserialize};
use someip::{FieldError, SomeIpHeader, MethodError, client::Client, config::Configuration, server::Server, tasks::ConnectionInfo};
use crate::{connection::SomeIPCodec, someip_codec::SomeIpPacket, trait_tryout::interface::HelloWorldError};
use bytes::{Bytes, BytesMut};
use futures::SinkExt;
use std::{fmt::Write, iter::FromIterator, net::SocketAddr, str, sync::{Arc, Mutex, RwLock}, time::Duration};
use tokio::runtime::Runtime;

use self::interface::HelloWorldServer;


mod interface {
    use generator_macros::*;
    use std::io;
    use serde::{Serialize, Deserialize};
    use thiserror::Error;




    #[derive(Serialize,Deserialize, Default)]
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
        #[error("Connection Error")]
        ConnectionError,
    }

    #[service(fields([1]value1:Field1,[2]value2:String, [3]value3: u32),
        events([1 =>10]value1:Event1, [2=>10]value2:String, [3=>10]value3: u32), 
        method_ids([1]echo_int, [2]echo_string, [3]no_reply))]
    pub trait HelloWorld {
        fn echo_int(&self, value: i32, value1: Field1) -> Result<i32, HelloWorldError>;
        fn echo_string(&self, value: String) -> Result<String, HelloWorldError>;
        fn no_reply(&self, value: Field1);
    }

    pub struct HelloWorldServer {

    }
}

#[derive(Serialize,Deserialize)]
struct InputParams {
    value : i32,
    value1: interface::Field1,
}

pub struct HelloWorldProxy {
    // the client handle
    client : someip::client::Client,
}

/* 
impl HelloWorldProxy {

    pub fn new(service_id: u16, client_id: u16, config: Configuration) -> Self {
        HelloWorldProxy {
            client : someip::client::Client::new(service_id, client_id, config),
        }
    }

    pub async fn echo_int(&self, value: i32, value1: interface::Field1) -> Result<i32, MethodError<interface::HelloWorldError>> {
        let input_params = InputParams {
            value,
            value1
        };

        let mut header = SomeIpHeader::default();
        header.set_method_id(1);

        //let params = Serialize!();
        let message = SomeIpPacket::new(header, Bytes::new());
        let res = self.client.call(message,std::time::Duration::from_millis(1000)).await;

        match res {
            Ok(someip::client::ReplyData::Completed(pkt)) => {
                match pkt.header().message_type {
                    someip::someip_codec::MessageType::Response => {
                        Ok(0)
                    }
                    someip::someip_codec::MessageType::Request => {
                        Err(MethodError::ConnectionError)
                    }
                    someip::someip_codec::MessageType::RequestNoReturn => {
                        Err(MethodError::ConnectionError)
                    }
                    someip::someip_codec::MessageType::Notification => {
                        Err(MethodError::ConnectionError)
                    }
                    someip::someip_codec::MessageType::Error => {
                        Err(MethodError::ConnectionError)
                    }
                }
            } 
            Ok(someip::client::ReplyData::Cancelled) => {
                Err(MethodError::ConnectionError)
            }
            Ok(someip::client::ReplyData::Pending) => {
                panic!("This should not happen");
            }
            Err(e) => {
                Err(MethodError::ConnectionError)
            }
        }
    }
}
*/


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
    fn echo_int(&self, value: i32, value1: interface::Field1) -> Result<i32, HelloWorldError> {
        Ok(value)
    }

    fn echo_string(&self, value: String) -> Result<String, HelloWorldError> {
        Ok(value)
    }

    fn set_value1(& self, _: interface::Field1) -> Result<(), FieldError> { Ok(()) }

    fn get_value1(&self) -> Result<&interface::Field1, FieldError> { todo!() }
    fn set_value2(&self, _: std::string::String) -> Result<(), FieldError> { Ok(())}
    fn get_value2(&self) -> Result<&std::string::String, FieldError> { todo!() }
    fn set_value3(&self, _: u32) -> Result<(), FieldError> { Ok(()) }
    fn get_value3(&self) -> Result<&u32, FieldError> { todo!() }

    fn no_reply(&self, value: interface::Field1) {
        println!("No reply");
    }

}

pub fn start_server() {

    println!("Size of Client: {}", std::mem::size_of::<someip::client::Client>());

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

        //tokio::time::sleep(Duration::from_millis(20)).await;
        async_std::task::sleep(Duration::from_millis(20)).await;

        let config = Configuration::default();
        let proxy = interface::HelloWorldProxy::new(45, 0, config);
        let addr = "127.0.0.1:8090".parse::<SocketAddr>().unwrap();
        let proxy_for_task = proxy.clone();
  
        tokio::spawn(async move { interface::HelloWorldProxy::run(proxy_for_task,addr).await});

        tokio::spawn(async move {
            // client stuff
           // let proxy = proxy.read().unwrap();
            let res = proxy.echo_string(String::from("Hello World")).await;
            println!("Return value: {:?}", res);
            let res = proxy.echo_string(String::from("Hello World2")).await;
            println!("Return value: {:?}", res);
            let res = proxy.no_reply(interface::Field1::default()).await;

            //let field1 = proxy.value1.get();
            //let err = proxy.value1.set(interface::Field1::default()).await;
            //proxy.value1.on_change(|v|{/*  changed value1 */});
            //proxy.on_event1(|e|{ /*  do something with the event*/})

        });
 
        println!("Sending terminate");
        tokio::time::sleep(Duration::from_millis(2000)).await;
        //let res = &mut handle.terminate().await;
    });
}