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

use core::panic;
use std::{
    collections::HashMap,
    io::{self, Error},
    net::SocketAddr,
    pin::Pin,
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc, Mutex, RwLock,
    },
    task::{Context, Poll, Waker},
};

use async_trait::async_trait;
use futures::{Future, SinkExt, StreamExt};

use tokio::{
    net::{TcpStream, UnixStream},
    sync::mpsc::{channel, Receiver, Sender},
};
use tokio_util::codec::Framed;

use crate::someip_codec::{MessageType, SomeIpPacket};
use crate::{config::Configuration, someip_codec::SomeIPCodec};

type PendingCalls = Arc<
    Mutex<
        HashMap<
            u32,
            (
                // the time at which the call was issued
                std::time::Instant,
                // the timeout for this call
                std::time::Duration,
                // the reply is stored here
                ReplyData,
                // The waker
                Option<Waker>,
            ),
        >,
    >,
>;

#[derive(Clone)]
pub struct Client {
    //config: Configuration,
    /// The pending calls structure.
    ///
    //pending_calls: PendingCalls,
    //dispatch_tx: Sender<DispatcherMessage>,
    //dispatch_rx: Arc<Mutex<Option<Receiver<DispatcherMessage>>>>,
    //service_id: u16,
    //client_id: u16,
    //session_id: AtomicU16,
    inner: Arc<RwLock<ClientInner>>,
}

impl Client {
    fn inner(&self) -> std::sync::RwLockReadGuard<'_, ClientInner> {
        let inner = self.inner.read().unwrap();
        inner
    }
}

struct ClientInner {
    config: Arc<Configuration>,
    pending_calls: PendingCalls,
    dispatch_tx: Sender<DispatcherMessage>,
    dispatch_rx: Arc<Mutex<Option<Receiver<DispatcherMessage>>>>,
    //service_id: u16,
    client_id: u16,
    session_id: AtomicU16,
}

impl ClientInner {
    pub fn new(client_id: u16, config: Arc<Configuration>) -> Self {
        let (dispatch_tx, dispatch_rx) = channel::<DispatcherMessage>(10);
        ClientInner {
            config,
            pending_calls: Arc::new(Mutex::new(HashMap::new())),
            dispatch_tx,
            dispatch_rx: Arc::new(Mutex::new(Some(dispatch_rx))),
            client_id,
            session_id: AtomicU16::new(0),
        }
    }
}

impl Client {
    pub fn new(client_id: u16, config: Arc<Configuration>) -> Self {
        //let (dispatch_tx, dispatch_rx) = channel::<DispatcherMessage>(10);
        Self {
            //config,
            //pending_calls: Arc::new(Mutex::new(HashMap::new())),
            //dispatch_tx,
            //dispatch_rx: Arc::new(Mutex::new(Some(dispatch_rx))),
            //client_id,
            //session_id: AtomicU16::new(0),
            //service_id,
            inner: Arc::new(RwLock::new(ClientInner::new(client_id, config))),
        }
    }

    pub async fn run(&self, to: SocketAddr) -> Result<(), io::Error> {
        let (config, dispatch_rx, pending_calls) = {
            let inner = self.inner();
            //let client = this.read().unwrap();
            let dispatch_rx = inner.dispatch_rx.lock().unwrap().take().unwrap();
            let config = inner.config.clone();
            (config, dispatch_rx, inner.pending_calls.clone())
        };
        tcp_client_dispatcher(&config, to, dispatch_rx, pending_calls).await
    }

    pub async fn run_uds(&self, on: std::os::unix::net::UnixStream) -> Result<(), io::Error> {
        let (config, dispatch_rx, pending_calls) = {
            let inner = self.inner();
            //let client = this.read().unwrap();
            let dispatch_rx = inner.dispatch_rx.lock().unwrap().take().unwrap();
            let config = inner.config.clone();
            (config, dispatch_rx, inner.pending_calls.clone())
        };
        uds_client_dispatcher(on, dispatch_rx, pending_calls).await
    }
    /*
    pub async fn run_static(&self, to: SocketAddr) -> Result<(), io::Error> {
        let (config, dispatch_rx, pending_calls) = {
            let inner = self.inner();
            //let client = this.read().unwrap();
            let dispatch_rx = inner.dispatch_rx.lock().unwrap().take().unwrap();
            let config = inner.config.clone();
            (config, dispatch_rx, inner.pending_calls.clone())
        };
        tcp_client_dispatcher(&config, to, dispatch_rx, pending_calls).await
    }
    */
}
//std::sync::Arc<std::sync::RwLock<someip::client::Client>>
impl Client {
    pub async fn call(
        &self,
        mut message: SomeIpPacket,
        timeout: std::time::Duration,
    ) -> Result<ReplyData, io::Error> {
        //let pending_calls = self.pending_calls.lock().unwrap();

        let (dispatch_tx, message, pending_calls, request_id) = {
            let inner = self.inner();

            let session_id =
                inner
                    .session_id
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |s| Some(s + 1));
            let request_id = (((inner.client_id as u32) << 16) | session_id.unwrap() as u32) as u32;
            message.header_mut().request_id = request_id;
            //message.header_mut().set_service_id(inner.service_id);

            log::debug!("Call:Pkt: {:?}", message.header());

            // add to pending call list
            {
                let mut pending_calls = inner.pending_calls.lock().unwrap();
                if let Some(_p) = pending_calls.insert(
                    request_id,
                    (std::time::Instant::now(), timeout, ReplyData::Pending, None),
                ) {
                    panic!(
                        "Fatal: Unexpected pending call with request_id: {}",
                        request_id
                    );
                }
            }

            // we need to clone the tx channel as we need to await and the future
            // could get scheduled on a different thread.
            let dispatch_tx = inner.dispatch_tx.clone();
            (
                dispatch_tx,
                message,
                inner.pending_calls.clone(),
                request_id,
            )
        };
        // everything below this is Sendable as we call await

        dispatch_tx
            .send(DispatcherMessage::Call(message))
            .await
            .map_err(|_e| {
                io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "Unable to send packet to dispatcher",
                )
            })?;
        let future = Reply::new(pending_calls.clone(), request_id);

        let timeout_future = tokio::time::sleep(timeout);
        tokio::pin!(timeout_future);
        tokio::select! {
            () = timeout_future => {
                log::debug!("timer elapsed");
                let mut pending_calls = pending_calls.lock().unwrap();
                if let Some(pending) = pending_calls.get_mut(&request_id) {
                    match pending.2 {
                        ReplyData::Pending => {
                            pending.2 = ReplyData::Cancelled;
                            if let Some(w) = pending.3.take() { w.wake() }
                            let data = pending_calls.remove_entry(&request_id).unwrap();
                            log::debug!("Removed pending call for {} {:?}", request_id, data);

                        }
                        _ => {
                            log::error!("Timeout but no pending call");
                        }
                    }
                    Ok(ReplyData::Cancelled)
                } else {
                    todo!()
                }

            },
            fut = future =>  {
                Ok(fut)
            }
        }
    }

    pub async fn call_noreply(&self, mut message: SomeIpPacket) -> Result<(), io::Error> {
        log::debug!("call_noreply");
        let (dispatch_tx, message) = {
            let inner = self.inner();
            //let this = this.read().unwrap();
            let session_id =
                inner
                    .session_id
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |s| Some(s + 1));
            let request_id = (((inner.client_id as u32) << 16) | session_id.unwrap() as u32) as u32;
            message.header_mut().request_id = request_id;
            //message.header_mut().set_service_id(inner.service_id);
            (inner.dispatch_tx.clone(), message)
        };
        // everything below is sendable
        dispatch_tx
            .send(DispatcherMessage::Call(message))
            .await
            .map_err(|_e| {
                io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "Unable to send packet to dispatcher",
                )
            })?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum ReplyData {
    Pending,
    Completed(SomeIpPacket),
    Cancelled,
}
/// A future for call replies
/// store the request ID for this call and the shared
/// handled to the list of pending calls
struct Reply(PendingCalls, u32);
impl Future for Reply {
    type Output = ReplyData;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let mut pending_calls = self.0.lock().unwrap();

        if let Some(pending) = pending_calls.get_mut(&self.1) {
            match pending.2 {
                ReplyData::Pending => {
                    pending.3 = Some(ctx.waker().clone());
                    Poll::Pending
                }
                ReplyData::Cancelled => Poll::Ready(ReplyData::Cancelled),
                ReplyData::Completed(_) => {
                    let data = pending_calls.remove_entry(&self.1).unwrap();
                    Poll::Ready(data.1 .2)
                }
            }
        } else {
            // there is no entry for this pending call.
            log::error!("Unexpected - no pending call entry for this future");
            Poll::Ready(ReplyData::Cancelled)
        }
    }
}

impl Reply {
    fn new(pending_calls: PendingCalls, request_id: u32) -> Self {
        Self(pending_calls, request_id)
    }
}

enum DispatcherMessage {
    Call(SomeIpPacket),
}

async fn uds_client_dispatcher(
    unix_stream: std::os::unix::net::UnixStream,
    mut dispatch_rx: Receiver<DispatcherMessage>,
    pending_calls: PendingCalls,
) -> Result<(), io::Error> {
    let stream = SomeIPCodec::create_uds_stream(unix_stream).unwrap();
    let (mut tx, mut rx) = stream.split();
    loop {
        tokio::select! {
           Some(message) = dispatch_rx.recv() => {
               match message {
                   DispatcherMessage::Call(pkt) => {
                       if pkt.header().message_type != MessageType::Request &&
                       pkt.header().message_type != MessageType::RequestNoReturn {
                           log::error!("Invalid request type. Considering this fatal");
                           break;
                       }

                       if let Err(e) = tx.send(pkt).await {
                           log::error!("Error sending Request Packet:{}",e);
                           break;
                       }
                   }
               }
           }
           // Received packets
           Some(Ok(pkt)) = rx.next() => {
               match pkt.header().message_type {
                   MessageType::Response | MessageType::Error => {
                       let request_id = pkt.header().request_id;
                       let mut pending_calls = pending_calls.lock().unwrap();
                       if let Some(mut pending) = pending_calls.get_mut(&request_id) {
                           pending.2 = ReplyData::Completed(pkt);
                           if let Some(w) = pending.3.take() { w.wake() }
                       } else {
                           log::info!("Response for request_id({}) received but it was not pending", request_id);
                       }
                   }
                   MessageType::Notification => {
                       todo!()
                   }
                   _ => {
                       log::error!("Unexpected packet type in client: {:?}", pkt.header().message_type);
                   }
               }
           }
        } // end tokio::select
    }
    Err(Error::new(io::ErrorKind::BrokenPipe, ""))
}

async fn tcp_client_dispatcher(
    config: &Configuration,
    to: SocketAddr,
    mut dispatch_rx: Receiver<DispatcherMessage>,
    pending_calls: PendingCalls,
) -> Result<(), io::Error> {
    loop {
        if let Ok(tcp_stream) = SomeIPCodec::new(config.max_packet_size_tcp)
            .connect(&to)
            .await
        {
            // println!("New client connection");
            //let mut session_id: u16 = 0;

            let (mut tx, mut rx) = tcp_stream.split();
            loop {
                tokio::select! {
                    Some(message) = dispatch_rx.recv() => {
                        match message {
                            DispatcherMessage::Call(pkt) => {
                                if pkt.header().message_type != MessageType::Request &&
                                pkt.header().message_type != MessageType::RequestNoReturn {
                                    log::error!("Invalid request type. Considering this fatal");
                                    panic!("Bad message type");
                                }

                                if let Err(e) = tx.send(pkt).await {
                                    log::error!("Error sending Request Packet:{}",e);
                                    continue
                                }

                            }
                        }
                    }
                    // Received packets
                    Some(Ok(pkt)) = rx.next() => {
                        match pkt.header().message_type {
                            MessageType::Response | MessageType::Error => {
                                let request_id = pkt.header().request_id;
                                let mut pending_calls = pending_calls.lock().unwrap();
                                if let Some(mut pending) = pending_calls.get_mut(&request_id) {
                                    pending.2 = ReplyData::Completed(pkt);
                                    if let Some(w) = pending.3.take() { w.wake() }
                                } else {
                                    log::info!("Response for request_id({}) received but it was not pending", request_id);
                                }
                            }
                            MessageType::Notification => {
                                todo!()
                            }
                            _ => {
                                log::error!("Unexpected packet type in client: {:?}", pkt.header().message_type);
                            }
                        }
                    }
                } // end tokio::select
            }
        }

        //  let (dispatch_tx, mut dispatch_rx) = channel::<DispatcherMessage>(1);
        // connection dropped, reconnect after a delay
        tokio::time::sleep(config.reconnection_delay).await;
    }
}

#[async_trait]
/// All proxies must implement this trait
pub trait Proxy : Sync + Send {
    fn get_dispatcher(&self) -> Client;
    async fn run(self, to: std::net::SocketAddr) -> Result<(), std::io::Error>;
    async fn run_uds(self, to: std::os::unix::net::UnixStream) -> Result<(), std::io::Error>;
}

pub trait ProxyConstruct {
    fn new(service_id: u16, client_id: u16, config: std::sync::Arc<Configuration>) -> Self;
    fn new_with_dispatcher(service_id: u16, client_dispatcher: Client) -> Self;
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use bytes::Bytes;
    use futures::future::BoxFuture;
    use someip_parse::SomeIpHeader;

    use super::*;
    use crate::{
        server::{Server, ServerRequestHandler},
        tasks::{ConnectionInfo, ConnectionMessage},
    };
    use simplelog::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_client_basic() {
        CombinedLogger::init(vec![TermLogger::new(
            LevelFilter::Debug,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Always,
        )])
        .unwrap();

        let config = Arc::new(Configuration::default());

        let client_config = config.clone();
        let client = Client::new(10, client_config);

        let rt = Runtime::new().unwrap();

        let to = "127.0.0.1:8099".parse::<SocketAddr>().unwrap();
        let at = to;
        println!("Client Test");
        let run_client = client.clone();

        let (tx, mut rx) = Server::create_notify_channel(1);

        struct TestService;

        #[async_trait]
        impl ServerRequestHandler for TestService {
            /*async fn handle(&mut self, message: SomeIpPacket) -> Option<SomeIpPacket> {
                println!("Packet received: {:?}", message);
                assert_eq!(message.header().service_id(), 0x45);
                assert_eq!(message.header().event_or_method_id(), 0x01);
                Some(SomeIpPacket::reply_packet_from(
                    message,
                    someip_parse::ReturnCode::Ok,
                    Bytes::new(),
                ))
            }*/

            fn get_handler(
                &self,
                message: SomeIpPacket,
            ) -> BoxFuture<'static, Option<SomeIpPacket>> {
                Box::pin(async move {
                    println!("Packet received: {:?}", message);
                    assert_eq!(message.header().service_id(), 0x45);
                    assert_eq!(message.header().event_or_method_id(), 0x01);
                    Some(SomeIpPacket::reply_packet_from(
                        message,
                        someip_parse::ReturnCode::Ok,
                        Bytes::new(),
                    ))
                })
            }
        }

        let server_config = config;
        let _result = rt.block_on(async {
            tokio::spawn(async move {
                loop {
                    if let Some(msg) = rx.recv().await {
                        match msg {
                            ConnectionInfo::NewTcpConnection((_sender, i)) => {
                                log::debug!("New connection from {}", i);
                            }
                            ConnectionInfo::ConnectionDropped(_i) => {}
                            ConnectionInfo::NewUdpConnection((sender, i)) => {
                                log::debug!("New UDP Connection from {}", i);
                                //test notification packet
                                let header = SomeIpHeader {
                                    message_type: MessageType::Notification,
                                    ..Default::default()
                                };
                                //header.message_type = MessageType::Notification;
                                let pkt = SomeIpPacket::new(header, Bytes::new());
                                let _res = sender
                                    .send(ConnectionMessage::SendUdpNotification((
                                        pkt,
                                        "127.0.0.1:8055".parse::<SocketAddr>().unwrap(),
                                    )))
                                    .await;
                            }
                            ConnectionInfo::UdpServerSocket(a) => {
                                assert_eq!(a, to);
                            }
                            ConnectionInfo::TcpServerSocket(a) => {
                                assert_eq!(a, to);
                            }
                        }
                    }
                }
            });

            struct Handler {
                inner: Arc<Mutex<TestService>>,
            }

            let handler: Arc<dyn ServerRequestHandler> = Arc::new(Handler {
                inner: Arc::new(Mutex::new(TestService {})),
            });

            //unsafe impl Sync for Handler {};

            impl ServerRequestHandler for Handler {
                fn get_handler(
                    &self,
                    message: SomeIpPacket,
                ) -> BoxFuture<'static, Option<SomeIpPacket>> {
                    let handle = self.inner.clone();
                    todo!()
                    //Box::pin(async move { dispatch(handle, message).await })
                }
            }

            tokio::spawn(async move {
                println!("Going to run server");
                let res = Server::serve(at, handler, server_config, 0x45, 1, 0, tx).await;
                println!("Server terminated");
                if let Err(e) = res {
                    println!("Server error:{}", e);
                }
            });

            tokio::spawn(async move {
                //fooo bar

                //let _res = Client::run_static(run_client, to).await; // client.run(at).await;
                let _res = run_client.run(to).await;
            });

            let mut header = SomeIpHeader::default();
            header.set_service_id(0x45);
            header.set_method_or_event_id(1);
            header.message_type = MessageType::Request;

            //let client = client.read().unwrap();

            let res = client
                .call(
                    SomeIpPacket::new(header, Bytes::new()),
                    std::time::Duration::from_millis(500),
                )
                .await;

            println!("Reply:{:?}", res);
        });
    }
}
