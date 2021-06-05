use core::panic;
use std::sync::{Arc, LockResult, RwLock, RwLockReadGuard};

use bincode::{deserialize, serialize};
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};
use someip_parse::{MessageType, SomeIpHeader};

use crate::error::PropertyError;
use crate::{Client, ReplyData, SomeIpPacket};
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

    pub async fn set(&mut self, val: T) -> Result<(), PropertyError> {
        self.set_with_timeout(val, std::time::Duration::from_secs(1))
            .await
    }

    pub async fn set_with_timeout(
        &mut self,
        val: T,
        timeout: std::time::Duration,
    ) -> Result<(), PropertyError> {
        let mut header = SomeIpHeader::default();
        header.set_event_id(self.field_id);
        header.message_type = MessageType::Request;
        let payload_raw = serialize(&val).unwrap();
        let packet = SomeIpPacket::new(header, Bytes::from(payload_raw));
        let res = self.client.call(packet, timeout).await;
        self.set_cached(val);

        match res {
            Ok(reply_data) => match reply_data {
                crate::ReplyData::Pending => {
                    panic!("Unexpected pending reply. Should be completed or cancelled")
                }
                crate::ReplyData::Completed(_) => Ok(()),
                crate::ReplyData::Cancelled => Err(PropertyError::Cancelled),
            },
            Err(_e) => Err(PropertyError::ConnectionError),
        }
    }

    pub async fn refresh(&mut self) -> Result<(), PropertyError> {
        self.refresh_with_timeout(std::time::Duration::from_secs(1))
            .await
    }

    pub async fn refresh_with_timeout(
        &mut self,
        timeout: std::time::Duration,
    ) -> Result<(), PropertyError> {
        let mut header = SomeIpHeader::default();
        header.set_event_id(self.field_id);
        header.message_type = MessageType::Request;
        let packet = SomeIpPacket::new(header, Bytes::new());
        if let Ok(reply) = self.client.call(packet, timeout).await {
            match reply {
                crate::ReplyData::Completed(packet) => {
                    let payload: T = deserialize(packet.payload()).unwrap();
                    self.set_cached(payload);
                    Ok(())
                }
                crate::ReplyData::Cancelled => todo!(),
                _ => Err(PropertyError::Cancelled),
            }
        } else {
            Err(PropertyError::ConnectionError)
        }
    }
}
