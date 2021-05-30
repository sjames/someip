use core::panic;
use std::{
    cell::RefCell,
    collections::HashMap,
    io,
    net::SocketAddr,
    pin::Pin,
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc, Mutex, RwLock,
    },
    task::{Context, Poll, Waker},
};

use bytes::Bytes;
use futures::{Future, SinkExt, StreamExt};
use someip_parse::SomeIpHeader;
use tokio::sync::mpsc::{channel, Receiver, Sender};

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

pub struct Client {
    config: Configuration,

    /// The pending calls structure.
    ///
    pending_calls: PendingCalls,
    dispatch_tx: Sender<DispatcherMessage>,
    dispatch_rx: Arc<Mutex<Option<Receiver<DispatcherMessage>>>>,
    service_id: u16,
    client_id: u16,
    session_id: AtomicU16,
}

impl Client {
    pub fn new(service_id: u16, client_id: u16, config: Configuration) -> Self {
        let (dispatch_tx, dispatch_rx) = channel::<DispatcherMessage>(10);
        Self {
            config,
            pending_calls: Arc::new(Mutex::new(HashMap::new())),
            dispatch_tx,
            dispatch_rx: Arc::new(Mutex::new(Some(dispatch_rx))),
            client_id,
            session_id: AtomicU16::new(0),
            service_id,
        }
    }

    pub async fn run(&self, to: SocketAddr) -> Result<(), io::Error> {
        let dispatch_rx = self.dispatch_rx.lock().unwrap().take().unwrap();
        tcp_client_dispatcher(&self.config, to, dispatch_rx, self.pending_calls.clone()).await
    }

    pub async fn run_static(this: Arc<RwLock<Client>>, to: SocketAddr) -> Result<(), io::Error> {
        let (config, dispatch_rx, pending_calls) = {
            let client = this.read().unwrap();
            let dispatch_rx = client.dispatch_rx.lock().unwrap().take().unwrap();
            let config = client.config.clone();
            (config, dispatch_rx, client.pending_calls.clone())
        };
        tcp_client_dispatcher(&config, to, dispatch_rx, pending_calls).await
    }
}
//std::sync::Arc<std::sync::RwLock<someip::client::Client>>
impl Client {
    pub async fn call(
        this: Arc<RwLock<Self>>,
        mut message: SomeIpPacket,
        timeout: std::time::Duration,
    ) -> Result<ReplyData, io::Error> {
        //let pending_calls = self.pending_calls.lock().unwrap();

        let this = this.read().unwrap();

        let session_id = this
            .session_id
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |s| Some(s + 1));
        let request_id = (((this.client_id as u32) << 16) | session_id.unwrap() as u32) as u32;
        message.header_mut().request_id = request_id;
        message.header_mut().set_service_id(this.service_id);

        println!("Pkt: {:?}", message.header());

        // add to pending call list
        {
            let mut pending_calls = this.pending_calls.lock().unwrap();
            if let Some(p) = pending_calls.insert(
                request_id,
                (std::time::Instant::now(), timeout, ReplyData::Pending, None),
            ) {
                panic!(
                    "Fatal: Unexpected pending call with request_id: {}",
                    request_id
                );
            }
        }

        this.dispatch_tx
            .send(DispatcherMessage::Call(message))
            .await
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "Unable to send packet to dispatcher",
                )
            })?;
        let future = Reply::new(this.pending_calls.clone(), request_id);
        Ok(future.await)
    }

    pub async fn call_noreply(
        this: Arc<RwLock<Self>>,
        mut message: SomeIpPacket,
    ) -> Result<(), io::Error> {
        log::debug!("call_noreply");
        let this = this.read().unwrap();
        let session_id = this
            .session_id
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |s| Some(s + 1));
        let request_id = (((this.client_id as u32) << 16) | session_id.unwrap() as u32) as u32;
        message.header_mut().request_id = request_id;
        message.header_mut().set_service_id(this.service_id);

        this.dispatch_tx
            .send(DispatcherMessage::Call(message))
            .await
            .map_err(|e| {
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
            panic!("Unexpected - no pending call entry for this future");
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
            println!("New client connection");
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
                                    pending.3.take().map(|w| w.wake());
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
                    // TODO: Handle timeout
                } // end tokio::select
            }
        }

        //  let (dispatch_tx, mut dispatch_rx) = channel::<DispatcherMessage>(1);
        // connection dropped, reconnect after a delay
        tokio::time::sleep(config.reconnection_delay).await;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::RwLock;

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

        let config = Configuration::default();

        let client_config = config.clone();
        let client = Arc::new(RwLock::new(Client::new(0x45, 10, client_config)));

        let rt = Runtime::new().unwrap();

        let to = "127.0.0.1:8090".parse::<SocketAddr>().unwrap();
        let at = to.clone();
        println!("Client Test");
        let run_client = client.clone();

        let (tx, mut rx) = Server::create_notify_channel(1);

        struct TestService;

        impl ServerRequestHandler for TestService {
            fn handle(&self, message: SomeIpPacket) -> Option<SomeIpPacket> {
                //println!("Packet received: {:?}", message);
                assert_eq!(message.header().service_id(), 0x45);
                assert_eq!(message.header().event_or_method_id(), 0x01);
                Some(SomeIpPacket::reply_packet_from(
                    message,
                    someip_parse::ReturnCode::Ok,
                    Bytes::new(),
                ))
            }
        }

        let server_config = config.clone();
        let _result = rt.block_on(async {
            tokio::spawn(async move {
                loop {
                    if let Some(msg) = rx.recv().await {
                        match msg {
                            ConnectionInfo::NewTcpConnection((sender, i)) => {
                                log::debug!("New connection from {}", i);
                            }
                            ConnectionInfo::ConnectionDropped(_i) => {}
                            ConnectionInfo::NewUdpConnection((sender, i)) => {
                                log::debug!("New UDP Connection");
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
                let res = Server::serve(at, service, server_config, 0x45, tx).await;
                println!("Server terminated");
                if let Err(e) = res {
                    println!("Server error:{}", e);
                }
            });

            tokio::spawn(async move {
                //fooo bar

                let _res = Client::run_static(run_client, to).await; // client.run(at).await;
            });

            let mut header = SomeIpHeader::default();
            header.set_service_id(0x45);
            header.set_method_or_event_id(1);
            header.message_type = MessageType::Request;

            //let client = client.read().unwrap();

            let res = Client::call(
                client,
                SomeIpPacket::new(header, Bytes::new()),
                std::time::Duration::from_millis(500),
            )
            .await;

            println!("Reply:{:?}", res);
        });
    }
}
