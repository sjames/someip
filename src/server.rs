// A SOME/IP server (provider)

use std::{io, net::SocketAddr, sync::Arc, sync::Mutex};
use tokio::sync::mpsc::{Receiver, Sender};

use tokio::sync::mpsc::channel;

use crate::config::Configuration;
use crate::someip_codec::SomeIpPacket;

use crate::tasks::*;

pub struct Server {}

impl Server {
    pub fn new() -> Self {
        Self {}
    }

    pub fn create_notify_channel(
        size: usize,
    ) -> (Sender<ConnectionInfo>, Receiver<ConnectionInfo>) {
        channel::<ConnectionInfo>(size)
    }
}

pub trait ServerRequestHandler {
    fn handle(&self, message: SomeIpPacket) -> Option<SomeIpPacket>;
}

impl Server {
    /// start serving.  This function doesn't return unless there is an
    /// unrecoverable error.
    pub async fn serve<'a>(
        at: SocketAddr,
        mut handler: Arc<Mutex<Box<impl ServerRequestHandler + Send + 'a>>>,
        config: Configuration,
        service_id: u16,
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
        handler: &mut Arc<Mutex<Box<impl ServerRequestHandler + Send + 'a>>>,
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
            fn handle(&self, message: SomeIpPacket) -> Option<SomeIpPacket> {
                println!("Packet received: {:?}", message);
                assert_eq!(message.header().service_id(), 0x45);
                assert_eq!(message.header().event_or_method_id(), 0x01);
                Some(message)
            }
        }

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
                                //test notification packet
                                let mut header = SomeIpHeader::default();
                                header.message_type = MessageType::Notification;
                                let pkt = SomeIpPacket::new(header, Bytes::new());
                                let res = sender
                                    .send(ConnectionMessage::SendUdpNotification((
                                        pkt,
                                        "127.0.0.1:8091".parse::<SocketAddr>().unwrap(),
                                    )))
                                    .await;
                            }
                        }
                    }
                }
            });

            tokio::spawn(async move {
                let test_service = Box::new(TestService {});
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
