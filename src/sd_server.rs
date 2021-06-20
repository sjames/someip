// Service discovery server
use crate::{
    config::Configuration,
    connection::SomeIPCodec,
    sd_messages::*,
    sd_server_sm::{SDServerStateMachine, SDServerStateMachineContainer, SMEvent, State},
    someip_codec::SomeIpPacket,
};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use someip_parse::MessageType;
use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio_util::time::{delay_queue::Key, DelayQueue};

pub enum SDServerMessage {
    InterfaceStatusChanged(bool),
    /// Service ID and the status of the service
    ServiceStatusChanged(u16, Service, bool),
}

pub struct Service {
    major_version: u8,
    minor_version: u32,
    addr: IpAddr,
    port: u16,
    proto: SDOptionTransportProto,
}

enum SDServerInternalMessage {
    OfferService(u16),
    StopOfferService(u16),
}

enum TimerCommand {
    SetTimer(u32, std::time::Duration),
    ResetTimer(u32),
}
enum TimerEvent {
    Timeout(u32),
}

async fn timer(mut timer_cmd_rx: Receiver<TimerCommand>, timer_tx: Sender<TimerEvent>) {
    let mut delay_queue: DelayQueue<u32> = DelayQueue::new();
    delay_queue.insert(0xff, std::time::Duration::from_secs(100));
    let mut pending_keys: Vec<(u32, Key)> = Vec::new();
    loop {
        tokio::select! {
            Some(Ok(value)) = delay_queue.next() => {
                let value = value.into_inner();
                if value == 0xff {
                    // always keep an entry to prevent from emptying the queue
                    delay_queue.insert(0xff, std::time::Duration::from_secs(100));
                } else if let Some(pos) = pending_keys.iter().position(|e| e.0 == value) {
                    pending_keys.swap_remove(pos);
                    let _res = timer_tx.send(TimerEvent::Timeout(value)).await;
                }
            }
            Some(cmd) = timer_cmd_rx.recv() => {
                match cmd {
                    TimerCommand::SetTimer(id, timeout) => {
                        let key = delay_queue.insert(id, timeout);
                        pending_keys.push((id,key));
                    },
                    TimerCommand::ResetTimer(id) => {
                        if let Some(pos) = pending_keys.iter().position(|e| e.0 == id) {
                            let _ = delay_queue.remove(&pending_keys[pos].1);
                            pending_keys.swap_remove(pos);

                        }
                    },
                }

            }

        }
    }
}

pub async fn sd_server_task(
    at: &SocketAddr,
    config: &Configuration,
    mut event_rx: Receiver<SDServerMessage>,
) -> Result<(), io::Error> {
    let mut udp_addr = SocketAddr::new(config.sd_multicast_ip, config.sd_port);

    //let mut services = Arc::new(Mutex::new(Vec::<(u16, SDServerStateMachine)>::new()));
    let mut services = Vec::<(u16, SDServerStateMachineContainer, Service)>::new();

    let ipv4_multicasts = match config.sd_multicast_ip {
        IpAddr::V4(addr) => Some(vec![(addr, Ipv4Addr::new(0, 0, 0, 0))]),
        IpAddr::V6(_) => None,
    };

    let ipv6_multicasts = match config.sd_multicast_ip {
        IpAddr::V4(addr) => None,
        IpAddr::V6(addr) => Some(vec![(addr, 0)]),
    };

    log::debug!("Starting Service Discovery server task");
    if let Ok(udp_stream) =
        SomeIPCodec::create_udp_stream(&udp_addr, ipv4_multicasts, ipv6_multicasts).await
    {
        let (mut udp_tx, mut rx) = udp_stream.split();
        let (dispatch_tx, mut dispatch_rx) = channel::<SDServerInternalMessage>(1);

        loop {
            tokio::select! {
                Some(Ok((packet,addr))) = rx.next() => {
                    if packet.header().is_someip_sd() {

                    } else {
                        log::error!("Non SD packet received from {}. Ignoring it", addr);
                    }
                }

                Some(msg) = event_rx.recv() => {
                    let tx = dispatch_tx.clone();
                    match msg {
                        SDServerMessage::InterfaceStatusChanged(enabled) => {
                            for service in services.iter_mut() {
                                service.1.next(SMEvent::IfStatusChanged(enabled));
                            }
                        },
                        SDServerMessage::ServiceStatusChanged(service_id,service,enabled) => {
                            if let Some(service) = services.iter_mut().find(|e|e.0 == service_id) {
                                // this service already exists
                                service.1.next(SMEvent::ServiceConfiguration(enabled));
                            } else {
                                let mut service_sm = SDServerStateMachineContainer::new(
                                    Box::new(|timer_id, duration| {
                                        println!("Setting timer {} for {:?}", timer_id, duration);
                                    }),
                                    Box::new(|timer_id| {
                                        println!("Resetting timer {} ", timer_id);
                                    }),
                                    Box::new(move || {
                                        println!("Send Offer");
                                        if let Err(e) = tx.blocking_send(SDServerInternalMessage::OfferService(service_id)) {
                                            panic!("cannot send offserservice message");
                                        }
                                    }),
                                    config.initial_delay_min,
                                    config.initial_delay_max,
                                    config.repetitions_max,
                                    config.repetition_base_delay,
                                    config.cyclic_offer_delay,
                                );
                                service_sm.next(SMEvent::ServiceConfiguration(enabled));
                                services.push((service_id,service_sm,service));
                            }

                        },
                    }
                }
                Some(msg) = dispatch_rx.recv() => {
                    match msg {
                        SDServerInternalMessage::OfferService(service_id) => {
                            if let Some(service) = services.iter().find(|e| e.0 == service_id) {
                                //create the service discovery message
                                let mut options : Vec::<SDOption> = Vec::new();
                                let addr_option = match &service.2.addr {
                                    IpAddr::V4(addr) => SDOption::Ipv4Endpoint {addr:*addr, transport: service.2.proto.clone(), port: service.2.port },
                                    IpAddr::V6(addr) => SDOption::Ipv6Endpoint {addr:*addr, transport: service.2.proto.clone(), port: service.2.port },
                                };
                                options.push(addr_option);
                                let mut message = SDMessage::default();
                                message.set_options(options);
                                let mut entry = ServiceEntry::default();
                                entry.set_service_entry_type(ServiceEntryType::Offer);
                                entry.set_service_id(service_id);
                                entry.set_major_version(service.2.major_version);
                                entry.set_minor_version(service.2.minor_version);
                                message.add_entry(entry, 0, 1, 0, 0).unwrap();
                                message.set_exp_initial_data_control(true);
                                message.set_reboot(true);

                                let pkt: SomeIpPacket = message.into();
                                if let Err(e) = udp_tx.send((pkt, udp_addr)).await {
                                    log::error!("Unable to send SomeIpPacket to {:?} due to {}", udp_addr,e);
                                }

                            } else {
                                panic!("Attempted offer service without service information");
                            }
                        },
                        SDServerInternalMessage::StopOfferService(_) => todo!(),
                    }
                }
            }
        }

        /*
        loop {
            tokio::select! {
                Some(Ok(packet)) = rx.next() => {
                        let (packet, addr) = packet;
                        if packet.header().service_id() != service_id {
                             log::error!(
                                 "(UDP:{})Invalid service ID({}) in packet for service {}\n packet:{:?}",udp_addr,
                                 packet.header().service_id(), service_id, packet.header()
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
        */
    } else {
        log::error!("Unable to bind to UDP");
    }
    Err(io::Error::new(
        io::ErrorKind::ConnectionAborted,
        "UDP server",
    ))
}
