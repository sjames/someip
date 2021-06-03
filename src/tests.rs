use crate::client::Client;
use crate::server::Server;
use crate::error::MethodError;

use super::*;
use std::sync::Mutex;
use tokio::runtime::Runtime;

use generator_macros::*;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use std::time;

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
    pub enum EchoError {
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
    pub trait EchoServer {
        fn echo_int(&self, value: i32, value1: Field1) -> Result<i32, EchoError>;
        fn echo_string(&self, value: String) -> Result<String, EchoError>;
        fn no_reply(&self, value: Field1);
    }

    pub struct EchoServerImpl {

    }

    impl ServerRequestHandler for EchoServerImpl {
        fn handle(&self, message: SomeIpPacket) -> Option<SomeIpPacket> {
            dispatcher::dispatch(self, message)
        }
    }
    impl EchoServer for EchoServerImpl {
        fn echo_int(&self, value: i32, value1: Field1) -> Result<i32, EchoError> {
            Ok(value)
        }
    
        fn echo_string(&self, value: String) -> Result<String, EchoError> {
            Ok(value)
        }
    
        fn set_value1(& self, _: Field1) -> Result<(), FieldError> { Ok(()) }
    
        fn get_value1(&self) -> Result<&Field1, FieldError> { todo!() }
        fn set_value2(&self, _: std::string::String) -> Result<(), FieldError> { Ok(())}
        fn get_value2(&self) -> Result<&std::string::String, FieldError> { todo!() }
        fn set_value3(&self, _: u32) -> Result<(), FieldError> { Ok(()) }
        fn get_value3(&self) -> Result<&u32, FieldError> { todo!() }
    
        fn no_reply(&self, value: Field1) {
            println!("No reply");
        }
    
    }

    impl Default for EchoServerImpl {
        fn default() -> Self {
            EchoServerImpl {
    
            }
        }
    }

    #[test]
    pub fn echo_tests() {
    
        let rt = Runtime::new().unwrap();
        let config = Configuration::default();
    
        let at = "127.0.0.1:8092".parse::<SocketAddr>().unwrap();
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
                let test_service = Box::new(EchoServerImpl::default());
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
            let proxy = EchoServerProxy::new(45, 0, config);
            let addr = "127.0.0.1:8092".parse::<SocketAddr>().unwrap();
            let proxy_for_task = proxy.clone();
      
            tokio::spawn(async move { EchoServerProxy::run(proxy_for_task,addr).await});
    
            tokio::spawn(async move {
                let res = proxy.echo_string(String::from("Hello World")).await;
                assert_eq!(res.unwrap(),String::from("Hello World"));

                let res = proxy.echo_string(String::from("Hello World2")).await;
                assert_eq!(res.unwrap(),String::from("Hello World2"));
         
                let res = proxy.no_reply(Field1::default()).await;
                assert_eq!(res, Ok(()));
    
                //let field1 = proxy.value1.get();
                //let err = proxy.value1.set(interface::Field1::default()).await;
                //proxy.value1.on_change(|v|{/*  changed value1 */});
                //proxy.on_event1(|e|{ /*  do something with the event*/})
    
            });
     
            println!("Sending terminate");
            tokio::time::sleep(Duration::from_millis(200)).await;
            //let res = &mut handle.terminate().await;
        });
    }
