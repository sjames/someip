use bytes::{Buf, Bytes, BytesMut};
use someip_parse::{ReadError, SomeIpHeader, SomeIpHeaderSlice};
use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;

pub use someip_parse::{MessageType, ReturnCode};
// default service discovery port
pub const SD_PORT: u32 = 30490;

#[derive(Debug, Clone)]
pub struct SomeIpPacket {
    payload: Bytes,
    header: SomeIpHeader,
}

impl SomeIpPacket {
    pub fn new(mut header: SomeIpHeader, payload: Bytes) -> Self {
        if header.tp_header.is_some() {
            header.length = payload.len() as u32 + 12;
        } else {
            header.length = payload.len() as u32 + 8;
        }
        Self { payload, header }
    }

    pub fn error_packet_from(received: SomeIpPacket, code: ReturnCode) -> Self {
        let mut header = received.header;
        header.message_type = MessageType::Error;
        header.return_code = code.into();
        let payload = Bytes::new();
        Self::new(header, payload)
    }

    pub fn reply_packet_from(received: SomeIpPacket, code: ReturnCode) -> Self {
        let mut header = received.header;
        header.message_type = MessageType::Response;
        header.return_code = code.into();
        let payload = Bytes::new();
        Self::new(header, payload)
    }

    pub fn header(&self) -> &SomeIpHeader {
        &self.header
    }
    pub fn header_mut(&mut self) -> &mut SomeIpHeader {
        &mut self.header
    }

    pub fn payload(&self) -> &Bytes {
        &self.payload
    }

    /// check of the packet is a reply type
    /// that is expected to be received by the client.
    pub fn is_expected_by_client(&self) -> bool {
        self.header.message_type == MessageType::Response
            || self.header.message_type == MessageType::Notification
    }
}
#[derive(Clone)]
pub struct SomeIPCodec {
    max_payload_length: u32,
}

impl SomeIPCodec {
    pub fn new(max_payload_length: u32) -> Self {
        SomeIPCodec { max_payload_length }
    }
}

impl Default for SomeIPCodec {
    fn default() -> Self {
        SomeIPCodec {
            max_payload_length: 4096,
        }
    }
}

impl Decoder for SomeIPCodec {
    type Item = SomeIpPacket;
    type Error = std::io::Error;
    fn decode(&mut self, buf: &mut BytesMut) -> std::io::Result<Option<Self::Item>> {
        println!("decode ({})", buf.len());

        // get atleast 8 bytes to cover the MessageID and Length
        if buf.len() < 8 {
            return Ok(None);
        }

        let payload_length = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
        if payload_length > self.max_payload_length {
            // return out of memory if packet size is too large
            return Err(std::io::Error::from(std::io::ErrorKind::OutOfMemory));
        }

        let packet_length = payload_length as usize + 8;

        if buf.len() < packet_length {
            return Ok(None);
        }

        let mut packet = buf.split_to(packet_length);

        let header = SomeIpHeaderSlice::from_slice(&packet);
        let header = match header {
            Err(e) => match e {
                ReadError::IoError(e) => return Err(e),
                ReadError::LengthFieldTooSmall(l) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("Length field too small:{}", l),
                    ))
                }
                ReadError::UnexpectedEndOfSlice(at) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("Unexpected end of slice at:{}", at),
                    ))
                }
                ReadError::UnknownMessageType(msg) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("Unknown message type:{}", msg),
                    ))
                }
                ReadError::UnsupportedProtocolVersion(v) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("Unsupported protocol version:{}", v),
                    ))
                }
            },
            Ok(h) => h.to_header(),
        };

        if header.tp_header.is_some() {
            packet.advance(someip_parse::SOMEIP_HEADER_LENGTH + someip_parse::TP_HEADER_LENGTH);
        } else {
            packet.advance(someip_parse::SOMEIP_HEADER_LENGTH);
        }

        log::debug!("Packet:{:?}", header);

        Ok(Some(SomeIpPacket {
            payload: packet.into(),
            header,
        }))
    }
}

impl Encoder<SomeIpPacket> for SomeIPCodec {
    type Error = std::io::Error;

    fn encode(&mut self, packet: SomeIpPacket, buf: &mut BytesMut) -> std::io::Result<()> {
        let mut tmp_buf = Vec::with_capacity(20); // = vec![0u8; 20]; //20 bytes worst case including TP header
        packet.header.write_raw(&mut tmp_buf).map_err(|_e| {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "Unable to write to packet".to_string(),
            )
        })?;

        buf.extend_from_slice(&tmp_buf);
        buf.extend(packet.payload);

        Ok(())
    }
}
