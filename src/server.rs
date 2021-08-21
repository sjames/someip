// A SOME/IP server (provider)

use std::{io, net::SocketAddr, sync::Arc, sync::Mutex};
use tokio::net::UnixStream;
use tokio::sync::mpsc::{Receiver, Sender};

use tokio::sync::mpsc::channel;

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

pub trait ServerRequestHandler {
    fn handle(&mut self, message: SomeIpPacket) -> Option<SomeIpPacket>;
}

impl Server {
    /// Serve services on a UDS connection.
    ///
    /// Since UDS doesn't have the concept of ports, we allow multiple service IDs on a single connection.
    /// the handlers variable is a slice of (service_id, handler, major_version, minor_version)
    /// This call does not return as long as the connection is active.
    pub async fn serve_uds(
        uds: UnixStream,
        mut handlers: Vec<(
            u16,                                              // service_id
            Arc<Mutex<Box<dyn ServerRequestHandler + Send>>>, // handler
            u8,                                               // major number
            u32,                                              // minor number
        )>,
    ) -> Result<(), io::Error> {
        let (dx_tx, mut dx_rx) = channel::<DispatcherCommand>(10);
        let uds_task = tokio::spawn(async move { uds_task(dx_tx, uds).await });

        // we make this a blocking receive to allow servers to block.
        let _server_task = tokio::task::spawn_blocking(move || {
            while let Some(command) = dx_rx.blocking_recv() {
                let (response, tx) = match command {
                    DispatcherCommand::DispatchUds(packet, tx) => {
                        if let Some(handler) = handlers.iter_mut().find_map(|e| {
                            if packet.header().service_id() == e.0 {
                                Some(e)
                            } else {
                                None
                            }
                        }) {
                            (Self::server_dispatch(&mut handler.1, packet), tx)
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
                if let Err(_e) = tx.blocking_send(DispatcherReply::ResponsePacket(response)) {
                    log::error!("Error sending response to UDS task");
                    break;
                }
            }
        });
        Ok(())
    }

    /// start serving.  This function doesn't return unless there is an
    /// unrecoverable error.
    pub async fn serve<'a>(
        at: SocketAddr,
        mut handler: Arc<Mutex<Box<dyn ServerRequestHandler + Send + 'a>>>,
        config: Configuration,
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
        let udp_task = tokio::spawn(async move {
            udp_task(dx_tx, &at, (&config).clone(), service_id, notify_tcp_tx).await
        });

        loop {
            if let Some(command) = dx_rx.recv().await {
                let (response, tx) = match command {
                    DispatcherCommand::DispatchUdp(packet, tx) => {
                        (Self::server_dispatch(&mut handler, packet), tx)
                    }
                    DispatcherCommand::DispatchTcp(packet, tx) => {
                        (Self::server_dispatch(&mut handler, packet), tx)
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

    fn server_dispatch<'a>(
        handler: &mut Arc<Mutex<Box<dyn ServerRequestHandler + Send + 'a>>>,
        packet: SomeIpPacket,
    ) -> Option<SomeIpPacket> {
        match packet.header().message_type {
            someip_parse::MessageType::Request => {
                if let Ok(mut handler) = handler.lock() {
                    handler.handle(packet)
                } else {
                    log::error!("Mutex poisoned?");
                    panic!("getting lock for handler");
                }
            }
            someip_parse::MessageType::RequestNoReturn => {
                if let Ok(mut handler) = handler.lock() {
                    handler.handle(packet);
                    None
                } else {
                    log::error!("Mutex poisoned?");
                    panic!("getting lock for handler");
                }
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
    use std::{fmt::Write, iter::FromIterator, net::SocketAddr, str, time::Duration};
    use tokio::runtime::Runtime;

    #[test]
    fn test_basic() {
        struct TestService;

        impl ServerRequestHandler for TestService {
            fn handle(&mut self, message: SomeIpPacket) -> Option<SomeIpPacket> {
                println!("Packet received: {:?}", message);
                assert_eq!(message.header().service_id(), 0x45);
                assert_eq!(message.header().event_or_method_id(), 0x01);
                Some(message)
            }
        }

        let rt = Runtime::new().unwrap();
        let config = Configuration::default();

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
                        }
                    }
                }
            });

            tokio::spawn(async move {
                let test_service: Box<dyn ServerRequestHandler + Send> = Box::new(TestService {});
                let mut service = Arc::new(Mutex::new(test_service));
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

            tx_connection.send(packet).await;

            println!("Sending terminate");
            //let res = &mut handle.terminate().await;
        });
    }
}
