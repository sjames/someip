pub use crate::someip_codec::SomeIPCodec;

use std::{
    io,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
};
use tokio::net::{TcpListener, TcpStream, UdpSocket, UnixStream};
use tokio_util::codec::Decoder;
use tokio_util::codec::Framed;
use tokio_util::udp::UdpFramed;

pub type TcpSomeIpConnection = Framed<TcpStream, SomeIPCodec>;
pub type UdpSomeIpConnection = UdpFramed<SomeIPCodec>;
pub type UdsSomeIpConnection = Framed<UnixStream, SomeIPCodec>;

impl SomeIPCodec {
    pub async fn connect(self, addr: &SocketAddr) -> Result<TcpSomeIpConnection, io::Error> {
        let tcp_stream = TcpStream::connect(addr).await?;
        log::debug!("Connected to {}", addr);
        Ok(self.framed(tcp_stream))
    }

    pub async fn listen(
        self,
        addr: &SocketAddr,
    ) -> Result<(TcpSomeIpConnection, SocketAddr), io::Error> {
        let listener = TcpListener::bind(addr).await?;

        match listener.accept().await {
            Ok((socket, addr)) => {
                let peer_addr = socket.peer_addr().unwrap();
                log::debug!("Connection accepted {:?} from {}", addr, peer_addr);
                Ok((self.framed(socket), peer_addr))
            }
            Err(e) => {
                log::error!("Error accepting {:?}", addr);
                Err(e)
            }
        }
    }

    /// Create a Stream for over UDP transport
    ///
    /// # Arguments
    ///
    /// * `addr` - The address to bind to
    /// * `multicast_v4` - A vector of multicast IPV4 (address,interface) tuples
    /// * `multicast_v6` - A vector of multicast IPV6 (address, interface_id) tuples.
    pub async fn create_udp_stream(
        addr: &SocketAddr,
        multicast_v4: Option<Vec<(Ipv4Addr, Ipv4Addr)>>,
        multicast_v6: Option<Vec<(Ipv6Addr, u32)>>,
    ) -> Result<UdpSomeIpConnection, io::Error> {
        if let Ok(socket) = UdpSocket::bind(addr).await {
            if let Some(multicast_v4) = multicast_v4 {
                for v4 in multicast_v4 {
                    socket.join_multicast_v4(v4.0, v4.1)?;
                }
            }

            if let Some(multicast_v6) = multicast_v6 {
                for v6 in multicast_v6 {
                    socket.join_multicast_v6(&v6.0, v6.1)?;
                }
            }

            // Maximum payload length for UDP is 1400 Bytes
            Ok(UdpFramed::new(socket, SomeIPCodec::new(1400)))
        } else {
            Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "Cannot bind UDP socket",
            ))
        }
    }

    pub fn create_uds_stream(uds: UnixStream) -> Result<UdsSomeIpConnection, io::Error> {
        Ok(Framed::new(uds, SomeIPCodec::new(1400)))
    }
}

#[cfg(test)]
mod tests {
    use core::panic;
    use std::fmt::Write;

    use crate::SomeIpPacket;

    use super::*;
    use bytes::BytesMut;
    use futures::{SinkExt, StreamExt};
    use someip_parse::SomeIpHeader;

    #[test]
    fn test_uds() {
        let rt = tokio::runtime::Runtime::new().unwrap();

        let _r = rt.block_on(async {
            let (tx, rx) = UnixStream::pair().unwrap();
            let mut stream = SomeIPCodec::create_uds_stream(rx).unwrap();

            rt.spawn(async {
                let mut sink = SomeIPCodec::create_uds_stream(tx).unwrap();
                let mut header = SomeIpHeader::default();
                header.set_service_id(42);
                header.set_method_or_event_id(67);
                let mut payload = BytesMut::with_capacity(10);
                payload.write_str("THIS IS A TEST").expect("payload write");
                let packet = SomeIpPacket::new(header, payload.freeze());
                let _res = sink.send(packet).await;
            });

            loop {
                if let Some(pkt) = stream.next().await {
                    if let Ok(packet) = pkt {
                        println!("Packet received:{:?}", &packet);
                        assert_eq!(packet.header().service_id(), 42);
                        assert_eq!(packet.header().event_or_method_id(), 67);
                        let mut payload = BytesMut::with_capacity(10);
                        payload.write_str("THIS IS A TEST").expect("payload write");
                        assert_eq!(payload, packet.payload());
                        break;
                    } else {
                        panic!("Packet not received");
                    }
                }
            }
        });
    }

    #[test]
    fn test_loopback() {
        let rt = tokio::runtime::Runtime::new().unwrap();

        let _result = rt.block_on(async {
            rt.spawn(async {
                println!("Initiating connection");
                let addr = "127.0.0.1:8094".parse::<SocketAddr>().unwrap();

                let stream = SomeIPCodec::default().connect(&addr).await;

                let (mut sink, _ins) = stream.unwrap().split();
                println!("Connected");

                let mut header = SomeIpHeader::default();
                header.set_service_id(42);
                header.set_method_or_event_id(67);
                let mut payload = BytesMut::with_capacity(10);
                payload.write_str("THIS IS A TEST").expect("payload write");
                let packet = SomeIpPacket::new(header, payload.freeze());

                let _res = sink.send(packet).await;
            });

            let addr = "127.0.0.1:8094".parse::<SocketAddr>().unwrap();

            let (stream, _addr) = SomeIPCodec::default().listen(&addr).await.unwrap();
            let (_sink, mut ins) = stream.split();
            println!("Connected!");

            let task = tokio::spawn(async move {
                loop {
                    if let Some(packet) = ins.next().await {
                        if let Ok(packet) = packet {
                            println!("Packet received:{:?}", &packet);
                            assert_eq!(packet.header().service_id(), 42);
                            assert_eq!(packet.header().event_or_method_id(), 67);
                            let mut payload = BytesMut::with_capacity(10);
                            payload.write_str("THIS IS A TEST").expect("payload write");
                            assert_eq!(payload, packet.payload());
                            break;
                        }
                    } else {
                        println!("Connection stopped");
                        break;
                    }
                }
            });

            task.await.unwrap();
        });
    }

    #[test]
    fn test_udp() {
        let rt = tokio::runtime::Runtime::new().unwrap();

        let _result = rt.block_on(async {
            rt.spawn(async {
                let addr = "0.0.0.0:4712".parse::<std::net::SocketAddr>().unwrap();

                let ipv4s = Vec::new();
                let ipv6 = Vec::new();
                let stream = SomeIPCodec::create_udp_stream(&addr, Some(ipv4s), Some(ipv6)).await;
                let (mut sink, _ins) = stream.unwrap().split();

                let mut header = SomeIpHeader::default();
                header.set_service_id(42);
                header.set_method_or_event_id(67);
                let mut payload = BytesMut::with_capacity(10);
                payload
                    .write_str("THIS IS A UDP TEST")
                    .expect("payload write");
                let packet = SomeIpPacket::new(header, payload.freeze());

                let dest_addr = "0.0.0.0:4713".parse::<SocketAddr>().unwrap();

                let res = sink.send((packet, dest_addr)).await;
                assert!(res.is_ok())
            });

            let addr = "0.0.0.0:4713".parse::<SocketAddr>().unwrap();

            let stream = SomeIPCodec::create_udp_stream(&addr, None, None).await;
            let (_sink, mut ins) = stream.unwrap().split();
            println!("Connected!");

            loop {
                if let Some(packet) = ins.next().await {
                    if let Ok((packet, addr)) = packet {
                        println!("Packet received:{:?}  from {}", &packet, &addr);
                        assert_eq!(packet.header().service_id(), 42);
                        assert_eq!(packet.header().event_or_method_id(), 67);
                        let mut payload = BytesMut::with_capacity(10);
                        payload
                            .write_str("THIS IS A UDP TEST")
                            .expect("payload write");
                        assert_eq!(payload, packet.payload());

                        break;
                    }
                } else {
                    println!("Connection stopped");
                    break;
                }
            }
        });
    }
}
