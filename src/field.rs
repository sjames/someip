use core::panic;
use std::sync::{Arc, LockResult, RwLock, RwLockReadGuard};

use bincode::{deserialize, serialize};
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};
use someip_parse::{MessageType, SomeIpHeader};

use crate::{Client, SomeIpPacket};
#[derive(Clone)]
pub struct Field<T> {
    val: Arc<RwLock<Option<T>>>,
    client: Client,
    field_id: u16,
}

impl<T> Field<T>
where
    T: Default + Serialize + DeserializeOwned,
{
    pub fn new(val: T, client: Client, field_id: u16) -> Self {
        Self {
            val: Arc::new(RwLock::new(None)),
            client,
            field_id,
        }
    }

    pub fn get_cached(&self) -> LockResult<RwLockReadGuard<'_, Option<T>>> {
        self.val.read()
    }

    pub fn set_cached(&self, val: T) {
        let mut writable = self.val.write().unwrap();
        *writable = Some(val);
    }

    pub async fn set(&mut self, val: T) {
        let mut header = SomeIpHeader::default();
        header.set_event_id(self.field_id);
        header.message_type = MessageType::Request;
        let payload_raw = serialize(&val).unwrap();
        let packet = SomeIpPacket::new(header, Bytes::from(payload_raw));
        let _res = self
            .client
            .call(packet, std::time::Duration::from_millis(100))
            .await;
        self.set_cached(val);
    }

    pub async fn get(&mut self) {
        let mut header = SomeIpHeader::default();
        header.set_event_id(self.field_id);
        header.message_type = MessageType::Request;
        let packet = SomeIpPacket::new(header, Bytes::new());
        if let Ok(reply) = self
            .client
            .call(packet, std::time::Duration::from_millis(100))
            .await
        {
            match reply {
                crate::ReplyData::Completed(packet) => {
                    let payload: T = deserialize(packet.payload()).unwrap();
                    self.set_cached(payload);
                }
                crate::ReplyData::Cancelled => todo!(),
                _ => panic!("Unexpected"),
            }
        }
    }
}
