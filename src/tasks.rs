use crate::{config::Configuration, connection::SomeIPCodec, someip_codec::SomeIpPacket};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use someip_parse::MessageType;
use std::{
    io,
    net::{IpAddr, SocketAddr},
};
use tokio::sync::mpsc::channel;
use tokio::sync::mpsc::Sender;

pub enum DispatcherCommand {
    Terminate,
    // dispatch a received UDP packet.  The sender to send the reply is sent as part of the
    // message
    DispatchUdp(SomeIpPacket, Sender<DispatcherReply>),
    // dispatch a received Tcp packet. The sender to send the reply is sent as a part of the
    // message
    DispatchTcp(SomeIpPacket, Sender<DispatcherReply>),
}

pub enum DispatcherReply {
    ResponsePacket(Option<SomeIpPacket>),
    ResponsePacketUdp(Option<SomeIpPacket>),
}

pub async fn tcp_client_task(
    tcp_dx_tx: Sender<DispatcherCommand>,
    at: &SocketAddr,
    config: Configuration,
) -> Result<(), io::Error> {
    let tcp_stream = SomeIPCodec::new(config.max_packet_size_tcp)
        .connect(at)
        .await?;
    let (mut tx, mut rx) = tcp_stream.split();
    let (dispatch_tx, mut dispatch_reply) = channel::<DispatcherReply>(1);
    loop {
        if let Some(Ok(packet)) = rx.next().await {
            if let Err(e) = tcp_dx_tx
                .send(DispatcherCommand::DispatchTcp(packet, dispatch_tx.clone()))
                .await
            {
                log::error!("Error sending to dispatcher:{}", e);
                break;
            } else {
                // wait for reply from dispatcher
                if let Some(r) = dispatch_reply.recv().await {
                    if let DispatcherReply::ResponsePacket(Some(packet)) = r {
                        if let Err(e) = tx.send(packet).await {
                            log::error!("Error sending response over TCP:{}", e);
                            break;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub enum ConnectionMessage {
    SendUdpNotification((SomeIpPacket, SocketAddr)),
    SendTcpNotification(SomeIpPacket),
}

pub enum ConnectionInfo {
    NewTcpConnection((Sender<ConnectionMessage>, SocketAddr)),
    NewUdpConnection((Sender<ConnectionMessage>, SocketAddr)),
    ConnectionDropped(IpAddr),
}

pub async fn tcp_server_task(
    tcp_dx_tx: Sender<DispatcherCommand>,
    at: &SocketAddr,
    config: Configuration,
    service_id: u16,
    notify_tcp_tx: Sender<ConnectionInfo>,
) -> Result<(), io::Error> {
    loop {
        log::debug!("Waiting for TCP connection from client");
        match SomeIPCodec::listen(SomeIPCodec::new(config.max_packet_size_tcp), &at).await {
            Ok((tcp_stream, addr)) => {
                // received a connection.

                // clone needed to move into the connection task below.  We can have multiple clients
                // connecting to the server
                let dx_tx = tcp_dx_tx.clone();
                let status_sender = notify_tcp_tx.clone();
                tokio::spawn(async move {
                    let (connection_control_tx, mut connection_control_rx) =
                        channel::<ConnectionMessage>(1);

                    if let Err(_e) = status_sender
                        .send(ConnectionInfo::NewTcpConnection((
                            connection_control_tx,
                            addr,
                        )))
                        .await
                    {
                        log::debug!("Unable to send NewTcpConnection Message");
                        return;
                    }

                    let (mut tx, mut rx) = tcp_stream.split();
                    log::debug!("New TCP connection from client");
                    let (dispatch_tx, mut dispatch_reply) = channel::<DispatcherReply>(1);

                    loop {
                        tokio::select! {
                                Some(Ok(packet)) = rx.next() => {
                                    if packet.header().service_id() != service_id {
                                        log::error!(
                                            "(TCP)Invalid service ID({}) in packet for service({})",
                                            packet.header().service_id(), service_id
                                        );
                                        if packet.header().message_type != MessageType::RequestNoReturn {
                                            let error = SomeIpPacket::error_packet_from(
                                                packet,
                                                someip_parse::ReturnCode::UnknownService,
                                                Bytes::new()
                                            );
                                            if let Err(e) = tx.send(error).await {
                                                log::error!("Error sending error reply {}", e);
                                                break;
                                            }
                                        }
                                        continue;
                                    }

                                    if let Err(e) = dx_tx
                                        .send(DispatcherCommand::DispatchTcp(packet, dispatch_tx.clone()))
                                        .await
                                    {
                                        log::error!("Error sending to dispatcher:{}", e);
                                        break;
                                    } else {
                                        // wait for reply from dispatcher
                                        if let Some(r) = dispatch_reply.recv().await {
                                            if let DispatcherReply::ResponsePacket(Some(packet)) = r {
                                                if let Err(_e) = tx.send(packet).await {
                                                    log::error!("Error sending response over TCP");
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }

                            Some(msg) = connection_control_rx.recv() => {
                                match msg {
                                    ConnectionMessage::SendTcpNotification(packet) => {
                                        if packet.header().message_type == MessageType::Notification {
                                            log::debug!("Sending notification packet");
                                            if let Err(_e) = tx.send(packet).await {
                                                log::error!("Error sending response over TCP");
                                                break;
                                            }

                                        } else {
                                            log::error!("Ignoring a packet that is not of Notification type");
                                        }
                                    }
                                     _ => {
                                         log::error!("Only TCP notifications can be sent over a TCP connection");
                                     }
                                }
                            }

                        }
                    }
                    let _e = tx.close().await;
                    log::debug!("TCP connection terminated")
                });
            }
            Err(e) => {
                log::error!("Listen error: {}", e)
            }
        }
        log::debug!("End TCP listening");
    }
}

pub async fn udp_task(
    dx_tx: Sender<DispatcherCommand>,
    at: &SocketAddr,
    _config: Configuration,
    service_id: u16,
    notify_ucp_tx: Sender<ConnectionInfo>,
) -> Result<(), io::Error> {
    let udp_addr = at.clone();

    if let Ok(udp_stream) = SomeIPCodec::create_udp_stream(&udp_addr, None, None).await {
        let (mut tx, mut rx) = udp_stream.split();
        let (dispatch_tx, mut dispatch_reply) = channel::<DispatcherReply>(1);

        let (connection_control_tx, mut connection_control_rx) = channel::<ConnectionMessage>(1);
        if let Err(_e) = notify_ucp_tx
            .send(ConnectionInfo::NewUdpConnection((
                connection_control_tx,
                (*at),
            )))
            .await
        {
            log::debug!("Unable to send NewTcpConnection Message");
            return Err(std::io::Error::new(
                io::ErrorKind::ConnectionAborted,
                "Unable to send connection notification",
            ));
        }
        loop {
            tokio::select! {
                Some(Ok(packet)) = rx.next() => {
                        let (packet, addr) = packet;
                        if packet.header().service_id() != service_id {
                            log::error!(
                                "(UDP)Invalid service ID({}) in packet for service {}",
                                packet.header().service_id(), service_id,
                            );
                            if packet.header().message_type != MessageType::RequestNoReturn {
                                let error = SomeIpPacket::error_packet_from(
                                    packet,
                                    someip_parse::ReturnCode::UnknownService,
                                    Bytes::new()
                                );
                                tx.send((error, addr)).await?;
                            }
                            continue;
                        }
                        if let Err(e) = dx_tx
                            .send(DispatcherCommand::DispatchUdp(packet, dispatch_tx.clone()))
                            .await
                        {
                            log::error!("Error sending to dispatcher:{}", e);
                            break;
                        } else if let Some(r) = dispatch_reply.recv().await {
                            if let DispatcherReply::ResponsePacket(Some(packet)) = r {
                                if let Err(_e) = tx.send((packet, addr)).await {
                                    log::error!("Error sending response over TCP");
                                    break;
                                }
                            }
                        } else {
                            log::error!("Unable to receive reply from dispatcher");
                            break;
                        }
                }
                Some(msg) = connection_control_rx.recv() => {
                    match msg {
                        ConnectionMessage::SendUdpNotification((packet,ip)) => {
                            if packet.header().message_type == MessageType::Notification {
                                log::debug!("Sending notification packet");
                                if let Err(_e) = tx.send((packet, ip)).await {
                                    log::error!("Error sending response over TCP");
                                    break;
                                }

                            } else {
                                log::error!("Ignoring a packet that is not of Notification type for {}", ip)
                            }
                        }
                        _ => {
                            log::error!("Only UDP notifications can be sent over Udp Connection ");
                        }
                    }
                }

            };
        }
    } else {
        log::error!("Unable to bind to UDP");
    }
    Err(io::Error::new(
        io::ErrorKind::ConnectionAborted,
        "UDP server",
    ))
}

/*
// receive notification events and forward to both TCP and UDP tasks
pub async fn notification_task(
    mut rx: Receiver<SomeIpPacket>,
    tx_udp: Option<Sender<SomeIpPacket>>,
    tx_tcp: Option<Sender<SomeIpPacket>>,
) -> Result<(), io::Error> {
    loop {
        if let Some(pkt) = rx.recv().await {
            let udp_pkt = pkt.clone();
            if let Some(ref tx_udp) = tx_udp {
                if let Err(e) = tx_udp.send(udp_pkt).await {
                    log::error!("UDP notify error : {}", e);
                    break;
                }
            }

            let tcp_pkt = pkt.clone();
            if let Some(ref tx_tcp) = tx_tcp {
                if let Err(e) = tx_tcp.send(tcp_pkt).await {
                    log::error!("UDP notify error : {}", e);
                    break;
                }
            }
        } else {
            log::error!("Notification task, recv failure");
            break;
        }
    }
    Ok(())
}
*/
