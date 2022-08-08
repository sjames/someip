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

// Service discovery server
use crate::{
    config::Configuration,
    connection::SomeIPCodec,
    sd_messages::*,
    sd_server_sm::{SDServerStateMachineContainer, SMEvent, State},
    someip_codec::SomeIpPacket,
};
use futures::{SinkExt, StreamExt};
use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
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
    SetTimer(u16, u16, std::time::Duration),
    ResetTimer(u16, u16),
}
enum TimerEvent {
    Timeout(u16, u16),
}

async fn timer(mut timer_cmd_rx: Receiver<TimerCommand>, timer_tx: Sender<TimerEvent>) {
    //  (timerid:u16, service_id:u16)
    let mut delay_queue: DelayQueue<(u16, u16)> = DelayQueue::new();
    delay_queue.insert((0xff, 0), std::time::Duration::from_secs(100));
    let mut pending_keys: Vec<(u16, u16, Key)> = Vec::new();
    loop {
        tokio::select! {
            Some(Ok(value)) = delay_queue.next() => {
                let value = value.into_inner();
                if value.0 == 0xff {
                    // always keep an entry to prevent from emptying the queue
                    //
                    delay_queue.insert((0xff,0), std::time::Duration::from_secs(100));
                } else if let Some(pos) = pending_keys.iter().position(|e| e.0 == value.0) {
                    pending_keys.swap_remove(pos);
                    let _res = timer_tx.send(TimerEvent::Timeout(value.0, value.1)).await;
                }
            }
            Some(cmd) = timer_cmd_rx.recv() => {
                match cmd {
                    TimerCommand::SetTimer(id, service_id,timeout) => {
                        let key = delay_queue.insert((id,service_id), timeout);
                        pending_keys.push((id,service_id,key));
                    },
                    TimerCommand::ResetTimer(id, service_id) => {
                        if let Some(pos) = pending_keys.iter().position(|e| e.0 == id && e.1 == service_id) {
                            let _ = delay_queue.remove(&pending_keys[pos].2);
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
    let udp_addr = SocketAddr::new(config.sd_multicast_ip, config.sd_port);

    //let mut services = Arc::new(Mutex::new(Vec::<(u16, SDServerStateMachine)>::new()));
    let mut services = Vec::<(u16, SDServerStateMachineContainer, Service)>::new();

    let ipv4_multicasts = match config.sd_multicast_ip {
        IpAddr::V4(addr) => Some(vec![(addr, Ipv4Addr::new(0, 0, 0, 0))]),
        IpAddr::V6(_) => None,
    };

    let ipv6_multicasts = match config.sd_multicast_ip {
        IpAddr::V4(_addr) => None,
        IpAddr::V6(addr) => Some(vec![(addr, 0)]),
    };

    log::debug!("Starting timer");
    let (timer_cmd_tx, timer_cmd_rx) = channel::<TimerCommand>(1);
    let (timer_tx, mut timer_rx) = channel::<TimerEvent>(1);
    let _timer_task = tokio::spawn(async move { timer(timer_cmd_rx, timer_tx).await });

    log::debug!("Starting Service Discovery server task");
    if let Ok(udp_stream) =
        SomeIPCodec::create_udp_stream(&udp_addr, ipv4_multicasts, ipv6_multicasts).await
    {
        let (mut udp_tx, mut rx) = udp_stream.split();
        let (dispatch_tx, mut dispatch_rx) = channel::<SDServerInternalMessage>(1);

        loop {
            let timer_cmd_set_tx = timer_cmd_tx.clone();
            let timer_cmd_reset_tx = timer_cmd_tx.clone();
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
                                    service_id,
                                    Box::new( move |timer_id, duration| {
                                        println!("Setting timer {} for {:?} service_id:{}", timer_id, duration,service_id);
                                        let _ = timer_cmd_set_tx.blocking_send(TimerCommand::SetTimer(timer_id,service_id,duration));

                                    }),
                                    Box::new(move |timer_id, service_id| {
                                        println!("Resetting timer {} ", timer_id);
                                        let _ = timer_cmd_reset_tx.blocking_send(TimerCommand::ResetTimer(timer_id,service_id));
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
                //Timer timeouts
                Some(cmd) = timer_rx.recv() => {
                    match cmd {
                        TimerEvent::Timeout(id,service_id) => {
                            log::debug!("Timeout");
                            if let Some(service) = services.iter_mut().find(|e|e.0 == service_id) {
                                // this service already exists
                                service.1.next(SMEvent::Timeout(id, service_id));
                            } else {
                                log::error!("Ignored unexpected timeout for service {}", service_id);
                            }
                        },
                    }
                }
            }
        }
    } else {
        log::error!("Unable to bind to UDP");
    }
    Err(io::Error::new(
        io::ErrorKind::ConnectionAborted,
        "UDP server",
    ))
}
