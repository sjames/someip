use crate::client::Client;
use crate::server::Server;
use crate::error::MethodError;

use super::*;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::{net::UnixStream, runtime::Runtime};

use generator_macros::*;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use std::time;
use async_trait::async_trait;

#[derive(Serialize,Deserialize, Default,Clone, Debug, PartialEq)]
pub struct SubField
{
    a : u32,
    b : String,
    c : HashMap<String,String>
}

#[derive(Serialize,Deserialize, Default, Clone, Debug, PartialEq)]
pub struct Field1 {
        a: u32,
        b: u16,
        c: String,
        d: Vec<String>,
        e: Vec<u64>,
        map : SubField,
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

    #[service(
        fields([1]value1:Field1,[2]value2:String, [3]value3: u32),
        events([1 ;10]value1:Event1, [2;10]value2:String, [3;10]value3: u32), 
        method_ids([1]echo_int, [2]echo_string, [3]no_reply),
        method_ids([4]echo_u64, [5]echo_struct)
    )]
    #[async_trait]
    pub trait EchoServer {
        fn echo_int(&mut self, value: i32) -> Result<i32, EchoError>;
        fn echo_string(&mut  self, value: String) -> Result<String, EchoError>;
        fn no_reply(&mut self, value: Field1);
        fn echo_u64(&mut self, value: u64) -> Result<u64, EchoError>;
        fn echo_struct(&mut self, value : Field1) -> Result<Field1, EchoError>;
    }

    pub struct EchoServerImpl {
        value1 : Field1,
    }

    impl ServerRequestHandler for EchoServerImpl {
        fn handle(&mut self, message: SomeIpPacket) -> Option<SomeIpPacket> {
            dispatcher::dispatch(self, message)
        }
    }

    impl EchoServer for EchoServerImpl {
        fn echo_int(&mut self, value: i32) -> Result<i32, EchoError> {
            Ok(value)
        }
    
        fn echo_string(&mut self, value: String) -> Result<String, EchoError> {
            std::thread::sleep(std::time::Duration::from_millis(1));
            Ok(value)
        }
    
        fn set_value1(&mut self, _: Field1) -> Result<(), FieldError> { Ok(()) }
    
        fn get_value1(&self) -> Result<&Field1, FieldError> { 
            Ok(&self.value1)
         }
        fn set_value2(&mut self, _: std::string::String) -> Result<(), FieldError> { Ok(())}
        fn get_value2(&self) -> Result<&std::string::String, FieldError> { todo!() }
        fn set_value3(&mut self, _: u32) -> Result<(), FieldError> { Ok(()) }
        fn get_value3(&self) -> Result<&u32, FieldError> { todo!() }
    
        fn no_reply(&mut self, value: Field1) {

        }

        fn echo_u64(&mut self, value: u64) -> Result<u64, EchoError> {
            Ok(value)
        }

        fn echo_struct(&mut self, value : Field1) -> Result<Field1, EchoError> {
            Ok(value)
        }
    
    }

    impl Default for EchoServerImpl {
        fn default() -> Self {
            let value1 = Field1 {
                a : 56678,
                ..Default::default()
            };
            
            EchoServerImpl {
                value1,
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
                let test_service : Box<dyn ServerRequestHandler + Send> = Box::new(EchoServerImpl::default());
                let service = Arc::new(Mutex::new(test_service));
                println!("Going to run server");
                let res = Server::serve(at, service, config, 45,1,0, tx).await;
                println!("Server terminated");
                if let Err(e) = res {
                    println!("Server error:{}", e);
                }
            });
    
            async_std::task::sleep(Duration::from_millis(20)).await;
    
            let config = Configuration::default();
            let mut proxy = EchoServerProxy::new(45, 0, config);
            let addr = "127.0.0.1:8092".parse::<SocketAddr>().unwrap();
            let proxy_for_task = proxy.clone();
      
            tokio::spawn(async move { EchoServerProxy::run(proxy_for_task,addr).await});
    
            let prop = CallProperties::default();

            let task = tokio::spawn(async move {
                for i in 1..25 {
                let res = proxy.echo_string(String::from("Hello World"), &prop).await;
                assert_eq!(res.unwrap(),String::from("Hello World"));

                let res = proxy.echo_string(String::from("Hello World2"),&prop).await;
                assert_eq!(res.unwrap(),String::from("Hello World2"));
         
                let res = proxy.echo_string(String::from("This should timeout"), &CallProperties::with_timeout(std::time::Duration::from_nanos(1))).await;
                //assert_eq!(res, Err(MethodError::ConnectionError));
        
                match res {
                    Ok(r) => {
                        panic!("This should have failed");
                    },
                    Err(e) => {

                    }
                }
    

                let res = proxy.no_reply(Field1::default(),&prop).await;
                assert_eq!(res, Ok(()));

                let res = proxy.echo_int(42i32,&prop).await;
                assert_eq!(42i32, res.unwrap());

                let res = proxy.echo_u64(42,&prop).await;
                assert_eq!(42u64, res.unwrap());

                proxy.value1.set(Field1::default()).await.unwrap();
                let _val = proxy.value1.refresh().await.unwrap();
                println!("Val: {:?}", proxy.value1.get_cached());

                
                

                let field = Field1 {
                    a: 75,
                    b: 56,
                    c:String::from("This is a string"),
                    d: vec![String::from("foo"), String::from("bar")],
                    e: vec![1,2,3,4,5,6,7],
                    map: SubField {
                        a: 5,
                        b: String::from("baz"),
                        c: HashMap::new(),
                    },
                };
                
                let returned = field.clone();
                let res = proxy.echo_struct(field,&prop).await;
                assert_eq!(returned, res.unwrap());
            }

    
            });
            let _ = task.await;

        });
    }

    #[test]
    pub fn echo_uds_tests() {
    
        let rt = Runtime::new().unwrap();
        let config = Configuration::default();
    
        let _result = rt.block_on(async {
    
            let (server,client) = UnixStream::pair().unwrap();
    
            tokio::spawn(async move {
                //let service : dyn ServerRequestHandler  = EchoServerImpl::default();

                let test_service : Box<dyn ServerRequestHandler + Send> = Box::new( EchoServerImpl::default());

                let service = Arc::new(Mutex::new(test_service));
                println!("Going to run server");
                let mut handlers = vec![(45, service, 1, 0)];
                let res = Server::serve_uds(server, handlers).await;
                println!("Server terminated");
                if let Err(e) = res {
                    println!("Server error:{}", e);
                }
            });
    
            async_std::task::sleep(Duration::from_millis(20)).await;
    
            let config = Configuration::default();
            let mut proxy = EchoServerProxy::new(45, 0, config);
            let proxy_for_task = proxy.clone();
  
            tokio::spawn(async move { EchoServerProxy::run_uds(proxy_for_task,client).await});
    
            let prop = CallProperties::default();

            let task = tokio::spawn(async move {
                for i in 1..25 {
                let res = proxy.echo_string(String::from("Hello World"), &prop).await;
                assert_eq!(res.unwrap(),String::from("Hello World"));

                let res = proxy.echo_string(String::from("Hello World2"),&prop).await;
                assert_eq!(res.unwrap(),String::from("Hello World2"));
         
                let res = proxy.echo_string(String::from("This should timeout"), &CallProperties::with_timeout(std::time::Duration::from_nanos(1))).await;
        
                match res {
                    Ok(r) => {
                        panic!("This should have failed");
                    },
                    Err(e) => {

                    }
                }
    

                let res = proxy.no_reply(Field1::default(),&prop).await;
                assert_eq!(res, Ok(()));

                let res = proxy.echo_int(42i32,&prop).await;
                assert_eq!(42i32, res.unwrap());

                let res = proxy.echo_u64(42,&prop).await;
                assert_eq!(42u64, res.unwrap());

                proxy.value1.set(Field1::default()).await.unwrap();
                let _val = proxy.value1.refresh().await.unwrap();
                //println!("Val: {:?}", proxy.value1.get_cached());

                
                

                let field = Field1 {
                    a: 75,
                    b: 56,
                    c:String::from("This is a string"),
                    d: vec![String::from("foo"), String::from("bar")],
                    e: vec![1,2,3,4,5,6,7],
                    map: SubField {
                        a: 5,
                        b: String::from("baz"),
                        c: HashMap::new(),
                    },
                };
                
                let returned = field.clone();
                let res = proxy.echo_struct(field,&prop).await;
                assert_eq!(returned, res.unwrap());
            }

    
            });
            let _ = task.await;

        });
    }
    

