/*
    Copyright 2021 Sojan James
    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at
        http://www.apache.org/licenses/LICENSE-2.0
    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.
*/

// A SOME/IP server

use async_trait::async_trait;
use futures::future::BoxFuture;
use std::{io, net::SocketAddr, sync::Arc};
use tokio::sync::mpsc::channel;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::config::Configuration;
use crate::someip_codec::SomeIpPacket;
use crate::tasks::{tcp_server_task, udp_task, uds_task};
use crate::{ConnectionInfo, DispatcherCommand, DispatcherReply};

pub struct Server {}

impl Default for Server {
    fn default() -> Self {
        Server {}
    }
}

impl Server {
    pub fn new() -> Self {
        Server::default()
    }

    pub fn create_notify_channel(
        size: usize,
    ) -> (Sender<ConnectionInfo>, Receiver<ConnectionInfo>) {
        channel::<ConnectionInfo>(size)
    }
}

pub struct ServerRequestHandlerEntry {
    pub name: &'static str,
    pub instance_id: u16,
    pub major_version: u8,
    pub minor_version: u32,
    pub handler: std::sync::Arc<dyn ServerRequestHandler>,
}

pub trait CreateServerRequestHandler {
    type Item;
    fn create_server_request_handler(
        server: std::sync::Arc<Self::Item>,
    ) -> Vec<ServerRequestHandlerEntry>;
}
pub trait ServerRequestHandler: Send + Sync {
    /// Return a boxed future that can be used to dispatch this message
    fn get_handler(&self, message: SomeIpPacket) -> BoxFuture<'static, Option<SomeIpPacket>>;
}
pub trait ServiceIdentifier {
    /// The service name for this service
    fn service_name() -> &'static str;
}

pub trait ServiceVersion {
    fn __major_version__() -> u8 {
        0
    }
    fn __minor_version__() -> u32 {
        0
    }
}

pub trait ServiceInstance {
    fn __instance_id__() -> u16 {
        0
    }
}

#[allow(clippy::type_complexity)]
impl Server {
    /// Serve services on a UDS connection.
    ///
    /// Since UDS doesn't have the concept of ports, we allow multiple service IDs on a single connection.
    /// the handlers variable is a slice of (service_id, handler, major_version, minor_version)
    /// This call does not return as long as the connection is active.
    pub async fn serve_uds(
        uds: std::os::unix::net::UnixStream,
        handlers: &[(
            u16,                           // service_id
            Arc<dyn ServerRequestHandler>, // handler
            u8,                            // major number
            u32,                           // minor number
        )],
    ) -> Result<(), io::Error> {
        let (dx_tx, mut dx_rx) = channel::<DispatcherCommand>(10);
        let _uds_task = tokio::spawn(async move { uds_task(dx_tx, uds).await });

        // Servers are also async and should not block
        while let Some(command) = dx_rx.recv().await {
            let (response, tx) = match command {
                DispatcherCommand::DispatchUds(packet, tx) => {
                    if let Some(handler) = handlers.iter().find_map(|e| {
                        if packet.header().service_id() == e.0 {
                            Some(e)
                        } else {
                            None
                        }
                    }) {
                        (Self::server_dispatch(handler.1.clone(), packet).await, tx)
                    } else {
                        panic!("{}", "unhandled service id");
                    }
                }
                DispatcherCommand::DispatchUdp(_, _) => {
                    panic!("{}", "UDP is not expected here");
                }
                DispatcherCommand::DispatchTcp(_, _) => {
                    panic!("{}", "TCP is not expected here");
                }
                DispatcherCommand::Terminate => {
                    log::debug!("Dispatcher terminating");
                    break;
                }
            };
            if let Err(_e) = tx.send(DispatcherReply::ResponsePacket(response)).await {
                log::error!("Error sending response to UDS task");
                break;
            }
        }
        Ok(())
    }

    /// start serving.  This function doesn't return unless there is an
    /// unrecoverable error.
    pub async fn serve<'a>(
        at: SocketAddr,
        handler: Arc<dyn ServerRequestHandler>,
        config: Arc<Configuration>,
        service_id: u16,
        major_version: u8,
        minor_version: u32,
        notify_tcp_tx: Sender<ConnectionInfo>,
    ) -> Result<(), io::Error> {
        let (dx_tx, mut dx_rx) = channel::<DispatcherCommand>(10);

        // cloning dispatcher handle as it needs to move into the TCP task
        let udp_config = config.clone();
        let tcp_dx_tx = dx_tx.clone();
        let tcp_notifier = notify_tcp_tx.clone();
        let tcp_task = tokio::spawn(async move {
            tcp_server_task(tcp_dx_tx, &at, udp_config, service_id, tcp_notifier).await
        });

        //cloning dx_tx tp move into the UDP task
        let dx_tx = dx_tx.clone();
        let udp_task =
            tokio::spawn(
                async move { udp_task(dx_tx, &at, config, service_id, notify_tcp_tx).await },
            );

        loop {
            if let Some(command) = dx_rx.recv().await {
                let (response, tx) = match command {
                    DispatcherCommand::DispatchUdp(packet, tx) => {
                        (Self::server_dispatch(handler.clone(), packet).await, tx)
                    }
                    DispatcherCommand::DispatchTcp(packet, tx) => {
                        (Self::server_dispatch(handler.clone(), packet).await, tx)
                    }
                    DispatcherCommand::Terminate => {
                        log::debug!("Dispatcher terminating");
                        break;
                    }
                    DispatcherCommand::DispatchUds(_, _) => {
                        panic!("{}", "UDS is not expected here");
                    }
                };
                if let Err(_e) = tx.send(DispatcherReply::ResponsePacket(response)).await {
                    log::error!("Error sending response to UDP task");
                    break;
                }
            } else {
                log::error!("Dispatcher task error");
                break;
            }
        }

        udp_task.abort();
        tcp_task.abort();

        Ok(())
    }

    async fn server_dispatch<'a>(
        handler: Arc<dyn ServerRequestHandler>,
        packet: SomeIpPacket,
    ) -> Option<SomeIpPacket> {
        match packet.header().message_type {
            someip_parse::MessageType::Request => {
                //let handler = if let Ok(handler) = handler.lock() {
                //    handler.get_handler(packet)
                //} else {
                //    log::error!("Mutex poisoned?");
                //    panic!("getting lock for handler");
                //};
                handler.get_handler(packet).await
            }
            someip_parse::MessageType::RequestNoReturn => {
                //let handler = if let Ok(handler) = handler.lock() {
                //    handler.get_handler(packet)
                //} else {
                //    log::error!("Mutex poisoned?");
                //    panic!("getting lock for handler");
                //};
                handler.get_handler(packet).await;
                None
            }
            someip_parse::MessageType::Notification => {
                log::error!("Server received Notification packet, dropped");
                None
            }
            someip_parse::MessageType::Response => {
                log::error!("Server received Response packet, dropped");
                None
            }
            someip_parse::MessageType::Error => {
                log::error!("Server received Error packet, dropped");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionMessage;
    use crate::{connection::SomeIPCodec, someip_codec::SomeIpPacket};
    use bytes::{Bytes, BytesMut};
    use futures::SinkExt;
    use someip_parse::{MessageType, SomeIpHeader};
    use std::{net::SocketAddr, time::Duration};
    use tokio::runtime::Runtime;

    #[test]
    fn test_basic() {
        struct TestService;

        #[async_trait]
        impl ServerRequestHandler for TestService {
            /*async fn handle(&mut self, message: SomeIpPacket) -> Option<SomeIpPacket> {
                println!("Packet received: {:?}", message);
                assert_eq!(message.header().service_id(), 0x45);
                assert_eq!(message.header().event_or_method_id(), 0x01);
                Some(message)
            }*/
            fn get_handler(
                &self,
                message: SomeIpPacket,
            ) -> BoxFuture<'static, Option<SomeIpPacket>> {
                Box::pin(async move {
                    println!("Packet received: {:?}", message);
                    assert_eq!(message.header().service_id(), 0x45);
                    assert_eq!(message.header().event_or_method_id(), 0x01);
                    Some(message)
                })
            }
        }

        let rt = Runtime::new().unwrap();
        let config = Arc::new(Configuration::default());

        let at = "127.0.0.1:8091".parse::<SocketAddr>().unwrap();
        println!("Test");
        let _result = rt.block_on(async {
            let (tx, mut rx) = Server::create_notify_channel(1);

            tokio::spawn(async move {
                loop {
                    if let Some(msg) = rx.recv().await {
                        match msg {
                            ConnectionInfo::NewTcpConnection((_sender, i)) => {
                                println!("New connection from {}", i);
                            }
                            ConnectionInfo::ConnectionDropped(_i) => {}
                            ConnectionInfo::NewUdpConnection((sender, i)) => {
                                //test notification packet
                                let header = SomeIpHeader {
                                    message_type: MessageType::Notification,
                                    ..Default::default()
                                };
                                let pkt = SomeIpPacket::new(header, Bytes::new());
                                let _res = sender
                                    .send(ConnectionMessage::SendUdpNotification((
                                        pkt,
                                        "127.0.0.1:9001".parse::<SocketAddr>().unwrap(),
                                    )))
                                    .await;
                            }
                            ConnectionInfo::UdpServerSocket(s) => {
                                assert_eq!(s, at);
                            }
                            ConnectionInfo::TcpServerSocket(s) => {
                                assert_eq!(s, at);
                            }
                        }
                    }
                }
            });

            tokio::spawn(async move {
                //let test_service: Box<dyn ServerRequestHandler> = Box::new(TestService {});
                let service = Arc::new(TestService {});
                println!("Going to run server");
                let res = Server::serve(at, service, config, 45, 1, 0, tx).await;
                println!("Server terminated");
                if let Err(e) = res {
                    println!("Server error:{}", e);
                }
            });

            tokio::time::sleep(Duration::from_millis(20)).await;

            let addr = "127.0.0.1:8091".parse::<SocketAddr>().unwrap();
            let mut tx_connection = SomeIPCodec::default().connect(&addr).await.unwrap();

            let mut header = SomeIpHeader::default();
            header.set_service_id(0x45);
            header.set_method_id(0x01);

            let payload = BytesMut::new().freeze();
            let packet = SomeIpPacket::new(header, payload);

            tx_connection.send(packet).await.unwrap();

            println!("Sending terminate");
            //let res = &mut handle.terminate().await;
        });
    }
}
