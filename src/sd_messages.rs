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

use bitvec::prelude::*;
use bytes::{BufMut, Bytes};
use someip_parse::SomeIpHeader;
use std::convert::TryInto;
use std::{
    convert::TryFrom,
    fmt::Display,
    io::ErrorKind,
    net::{Ipv4Addr, Ipv6Addr},
};

use crate::SomeIpPacket;

#[derive(PartialEq, Debug, Clone)]
pub struct SDMessage {
    header: SomeIpHeader,
    entries: Vec<ServiceEntry>,
    options: Vec<SDOption>,
    flags_reserved: u32,
}

impl SDMessage {
    pub fn set_options(&mut self, options: Vec<SDOption>) {
        self.options = options;
    }

    pub fn is_reboot(&self) -> bool {
        let flags: &BitSlice<u32, Msb0> = self.flags_reserved.view_bits();
        flags[0]
    }

    pub fn set_reboot(&mut self, reboot: bool) {
        let flags: &mut BitSlice<u32, Msb0> = self.flags_reserved.view_bits_mut();
        flags.set(0, reboot);
    }

    pub fn is_unicast(&self) -> bool {
        let flags: &BitSlice<u32, Msb0> = self.flags_reserved.view_bits();
        flags[1]
    }
    pub fn set_unicast(&mut self, unicast: bool) {
        let flags: &mut BitSlice<u32, Msb0> = self.flags_reserved.view_bits_mut();
        flags.set(1, unicast);
    }
    pub fn is_exp_initial_data_control(&self) -> bool {
        let flags: BitArray<u32, Msb0> = BitArray::from(self.flags_reserved);
        flags[2]
    }

    pub fn set_exp_initial_data_control(&mut self, initial_data_ctrl: bool) {
        let flags: &mut BitSlice<u32, Msb0> = self.flags_reserved.view_bits_mut();
        flags.set(2, initial_data_ctrl);
    }

    /// Add a service Entry. The configs index and run is specified here.
    /// Will return error if the options array does not have the corresponding entries.
    pub fn add_entry(
        &mut self,
        mut entry: ServiceEntry,
        config_index1: u8,
        config_run1: u8,
        config_index2: u8,
        config_run2: u8,
    ) -> Result<(), std::io::Error> {
        if config_index1 > 0xF || config_run1 > 0xF || config_index2 > 0xF || config_run2 > 0xF {
            return Err(std::io::Error::from(ErrorKind::InvalidInput));
        }

        if config_run1 > 0 && (config_index1 + config_run1) as usize >= self.options.len() {
            log::error!("config index+run1 is out of range. Options entries don't exist");
            return Err(std::io::Error::from(ErrorKind::NotFound));
        }

        if config_run2 > 0 && (config_index2 + config_run2) as usize >= self.options.len() {
            log::error!("config index+run2 is out of range. Options entries don't exist");
            return Err(std::io::Error::from(ErrorKind::NotFound));
        }

        entry.set_index1_options(config_index1, config_run1);
        entry.set_index2_options(config_index2, config_run2);

        self.entries.push(entry);

        Ok(())
    }
}

impl Default for SDMessage {
    fn default() -> Self {
        SDMessage {
            header: SomeIpHeader {
                message_id: 0xFFFF_8100,
                interface_version: 0x1,
                message_type: someip_parse::MessageType::Notification,
                length: 20, // Length field for empty SDMessage is 20 bytes
                ..Default::default()
            },
            entries: Vec::new(),
            options: Vec::new(),
            flags_reserved: 0,
        }
    }
}

impl From<SDMessage> for SomeIpPacket {
    fn from(msg: SDMessage) -> Self {
        let mut payload: Vec<u8> = Vec::new();
        payload.put_u32(msg.flags_reserved);
        payload.put_u32(msg.entries.len() as u32); // number of entries
        for entry in msg.entries {
            let e_raw = entry.as_buffer();
            payload.extend(e_raw);
        }

        let mut option_raw = Vec::new();
        for option in msg.options {
            let o_raw: Vec<u8> = option.into();
            option_raw.extend(o_raw);
        }
        payload.put_u32(option_raw.len() as u32); //Length of options bytes
        payload.extend(option_raw);

        SomeIpPacket::new(msg.header, Bytes::from(payload))
    }
}

impl TryFrom<SomeIpPacket> for SDMessage {
    type Error = ();

    fn try_from(pkt: SomeIpPacket) -> Result<Self, Self::Error> {
        if !pkt.header().is_someip_sd()
            || pkt.header().interface_version != 1
            || pkt.header().message_type != someip_parse::MessageType::Notification
        {
            return Err(());
        }
        let header = pkt.header().clone();
        let payload = pkt.payload();

        //println!("Pkt Header payload len: {}", pkt.header().length);
        //println!("Payload length:{}", payload.len());

        let flags_reserved = u32::from_be_bytes(payload[0..4].try_into().unwrap());
        let num_entries = u32::from_be_bytes(payload[4..4 + 4].try_into().unwrap()) as usize;
        let option_length_index = 8 + num_entries * 16;
        if payload.len() < option_length_index {
            log::error!("Invalid packet. Not enough bytes for service entries");
            return Err(());
        }
        let entries_slice = &payload[8..option_length_index]; // each entry is 16 bytes long

        let mut entries = Vec::new();

        for entry in entries_slice.chunks(16) {
            if let Ok(entry) = ServiceEntry::try_from(entry) {
                entries.push(entry);
            } else {
                log::error!("Failed to parse ServiceENtry");
                return Err(());
            }
        }

        if entries.len() != num_entries {
            log::error!("Incorrect number of entries");
            return Err(());
        }

        let options_slice = &payload[option_length_index..];
        let options_length_in_bytes =
            u32::from_be_bytes(options_slice[0..4].try_into().unwrap()) as usize;
        let options_buffer_slice = &options_slice[4..];

        if options_length_in_bytes != options_buffer_slice.len() {
            log::error!(
                "Inconsistent options length field expected:{} actual:{}",
                options_length_in_bytes,
                options_buffer_slice.len()
            );
            return Err(());
        }

        let mut options = Vec::new();

        let mut option_start_index = 0;
        loop {
            if option_start_index + 2 > options_buffer_slice.len() {
                break;
            }
            let len = u16::from_be_bytes(
                options_buffer_slice[option_start_index..option_start_index + 2]
                    .try_into()
                    .unwrap(),
            ) as usize;
            let option_end_index = option_start_index + len + 3;
            let option_slice = &options_buffer_slice[option_start_index..option_end_index];
            if let Ok(option) = SDOption::try_from(option_slice) {
                options.push(option);
            } else {
                log::error!("Error parsing SDOption");
                return Err(());
            }
            option_start_index = option_end_index;
        }

        Ok(SDMessage {
            header,
            entries,
            options,
            flags_reserved,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum SDOptionTransportProto {
    UDP,
    TCP,
}
#[derive(Debug, PartialEq, Clone)]
pub struct ConfigurationKey(String);

impl ConfigurationKey {
    pub fn new(name: &str) -> Self {
        if !name.is_ascii() {
            panic!("Only ascii names are allowed");
        } else {
            ConfigurationKey(String::from(name))
        }
    }
}

impl Display for ConfigurationKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug,PartialEq,Clone)]
#[rustfmt::skip]
pub enum SDOption {
    Configurations( Vec<(ConfigurationKey, String)>),
    LoadBalancing { priority: u16, weight: u16 },
    Ipv4Endpoint { addr : Ipv4Addr , transport : SDOptionTransportProto, port: u16},
    Ipv6Endpoint { addr : Ipv6Addr , transport : SDOptionTransportProto, port: u16},
    Ipv4Multicast { addr: Ipv4Addr, port: u16},
    Ipv6Multicast { addr: Ipv6Addr, port: u16},
    Ipv4SDEndpoint {addr : Ipv4Addr, port: u16},
    Ipv6SDEndpoint {addr : Ipv6Addr, port: u16},
}

impl TryFrom<&[u8]> for SDOption {
    type Error = ();

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let length: usize = u16::from_be_bytes(data[0..2].try_into().unwrap()) as usize;
        if data.len() != length + 3 {
            log::error!("Invalid length in packet");
            return Err(());
        }

        let ty: u8 = data[2];
        let packet = &data[4..];
        match ty {
            0x01 => {
                // Configuration Option
                let configs = parse_configuration_string(&packet[..length - 1]);
                Ok(SDOption::Configurations(configs))
            }
            0x02 => {
                // load balancing
                if length != 5 {
                    Err(())
                } else {
                    let priority = u16::from_be_bytes(data[4..6].try_into().unwrap());
                    let weight = u16::from_be_bytes(data[6..8].try_into().unwrap());
                    Ok(SDOption::LoadBalancing { priority, weight })
                }
            }
            0x04 => {
                // IpV4Endpoint
                if length != 9 {
                    Err(())
                } else {
                    let address = u32::from_be_bytes(data[4..8].try_into().unwrap());
                    let l4_proto = match data[9] {
                        0x6 => SDOptionTransportProto::TCP,
                        0x11 => SDOptionTransportProto::UDP,
                        _ => return Err(()),
                    };
                    let port = u16::from_be_bytes(data[10..12].try_into().unwrap());
                    Ok(SDOption::Ipv4Endpoint {
                        addr: Ipv4Addr::from(address),
                        transport: l4_proto,
                        port,
                    })
                }
            }
            0x06 => {
                // IpV6Endpoint
                if length != 0x15 {
                    Err(())
                } else {
                    let address = u128::from_be_bytes(data[4..4 + 16].try_into().unwrap());
                    let l4_proto = match data[21] {
                        0x6 => SDOptionTransportProto::TCP,
                        0x11 => SDOptionTransportProto::UDP,
                        _ => return Err(()),
                    };
                    let port = u16::from_be_bytes(data[22..24].try_into().unwrap());
                    Ok(SDOption::Ipv6Endpoint {
                        addr: Ipv6Addr::from(address),
                        transport: l4_proto,
                        port,
                    })
                }
            }

            0x14 => {
                // Ipv4 Multicast option
                if length != 0x9 {
                    Err(())
                } else {
                    let address = u32::from_be_bytes(data[4..8].try_into().unwrap());
                    let _l4_proto = match data[9] {
                        0x11 => SDOptionTransportProto::UDP, // always UDP
                        _ => return Err(()),
                    };
                    let port = u16::from_be_bytes(data[10..12].try_into().unwrap());
                    Ok(SDOption::Ipv4Multicast {
                        addr: Ipv4Addr::from(address),
                        port,
                    })
                }
            }

            0x16 => {
                // IpV6Multicast
                if length != 0x15 {
                    Err(())
                } else {
                    let address = u128::from_be_bytes(data[4..4 + 16].try_into().unwrap());
                    let _l4_proto = match data[21] {
                        0x11 => SDOptionTransportProto::UDP,
                        _ => return Err(()),
                    };
                    let port = u16::from_be_bytes(data[22..24].try_into().unwrap());
                    Ok(SDOption::Ipv6Multicast {
                        addr: Ipv6Addr::from(address),
                        port,
                    })
                }
            }

            0x24 => {
                // Ipv4 SD Endpoint
                if length != 0x9 {
                    Err(())
                } else {
                    let address = u32::from_be_bytes(data[4..8].try_into().unwrap());
                    let _l4_proto = match data[9] {
                        0x11 => SDOptionTransportProto::UDP, // always UDP
                        _ => return Err(()),
                    };
                    let port = u16::from_be_bytes(data[10..12].try_into().unwrap());
                    Ok(SDOption::Ipv4SDEndpoint {
                        addr: Ipv4Addr::from(address),
                        port,
                    })
                }
            }

            0x26 => {
                // Ipv6 SD Endpoint
                if length != 0x15 {
                    Err(())
                } else {
                    let address = u128::from_be_bytes(data[4..4 + 16].try_into().unwrap());
                    let _l4_proto = match data[21] {
                        0x11 => SDOptionTransportProto::UDP,
                        _ => return Err(()),
                    };
                    let port = u16::from_be_bytes(data[22..24].try_into().unwrap());
                    Ok(SDOption::Ipv6SDEndpoint {
                        addr: Ipv6Addr::from(address),
                        port,
                    })
                }
            }
            _ => Err(()),
        }
    }
}

fn parse_configuration_string(data: &[u8]) -> Vec<(ConfigurationKey, String)> {
    let mut current_pos: usize = 0;
    let mut entries: Vec<(ConfigurationKey, String)> = Vec::new();
    loop {
        let len = data[current_pos] as usize;
        if len == 0 {
            break;
        }

        if len + current_pos >= data.len() {
            println!(
                "Invalid data in buffer : current_pos({})  total:{}",
                current_pos,
                data.len()
            );
            break;
        }

        let record = &data[current_pos + 1..(len + current_pos + 1)];

        if let Ok(record) = String::from_utf8(record.to_owned()) {
            let mut iter = record.split('=');
            if let Some(key) = iter.next() {
                if let Some(value) = iter.next() {
                    entries.push((ConfigurationKey::new(key), String::from(value)));
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            // invalid string
            break;
        }
        current_pos += len + 1;
    }
    entries
}

#[allow(clippy::from_over_into)]
impl Into<Vec<u8>> for SDOption {
    fn into(self) -> Vec<u8> {
        let mut option_bytes = Vec::new();

        match self {
            SDOption::Configurations(configs) => {
                let mut cfg_entry: Vec<u8> = Vec::new();
                for config in configs {
                    let cfg_string = format!("{}={}", config.0, config.1);
                    let len = cfg_string.len() as u8;
                    cfg_entry.push(len);
                    cfg_entry.extend(cfg_string.as_bytes());
                }
                cfg_entry.push(0u8);

                let config_option_len: u16 = cfg_entry.len() as u16 + 1; // length includes the reserved byte
                let len_bytes = config_option_len.to_be_bytes();

                //let mut config_option_bytes = Vec::new();
                option_bytes.extend(len_bytes);
                option_bytes.push(0x01); // type
                option_bytes.push(0x0); // reserved
                option_bytes.extend(cfg_entry);
            }
            SDOption::LoadBalancing { priority, weight } => {
                option_bytes.put_u16(5);
                option_bytes.push(0x2);
                option_bytes.push(0); // reserved
                option_bytes.put_u16(priority);
                option_bytes.put_u16(weight);
            }
            SDOption::Ipv4Endpoint {
                addr,
                transport,
                port,
            } => {
                option_bytes.put_u16(9);
                option_bytes.push(4);
                option_bytes.push(0); //reserved
                option_bytes.put_u32(addr.into());
                option_bytes.push(0); //reserved
                if let SDOptionTransportProto::TCP = transport {
                    option_bytes.push(0x6);
                } else {
                    option_bytes.push(0x11);
                }
                option_bytes.put_u16(port);
            }
            SDOption::Ipv6Endpoint {
                addr,
                transport,
                port,
            } => {
                option_bytes.put_u16(0x15);
                option_bytes.push(0x6);
                option_bytes.push(0); //reserved
                option_bytes.put_u128(addr.into());
                option_bytes.push(0); //reserved
                if let SDOptionTransportProto::TCP = transport {
                    option_bytes.push(0x6);
                } else {
                    option_bytes.push(0x11);
                }
                option_bytes.put_u16(port);
            }
            SDOption::Ipv4Multicast { addr, port } => {
                option_bytes.put_u16(0x9);
                option_bytes.push(0x14);
                option_bytes.push(0); // reserved
                option_bytes.put_u32(addr.into());
                option_bytes.push(0); //reserved
                option_bytes.push(0x11); //always UDP
                option_bytes.put_u16(port);
            }
            SDOption::Ipv6Multicast { addr, port } => {
                option_bytes.put_u16(0x15);
                option_bytes.push(0x16); //type
                option_bytes.push(0); //reserved
                option_bytes.put_u128(addr.into());
                option_bytes.push(0); //reserved
                option_bytes.push(0x11); //always UDP
                option_bytes.put_u16(port);
            }
            SDOption::Ipv4SDEndpoint { addr, port } => {
                option_bytes.put_u16(0x9);
                option_bytes.push(0x24);
                option_bytes.push(0); // reserved
                option_bytes.put_u32(addr.into());
                option_bytes.push(0); //reserved
                option_bytes.push(0x11); //always UDP
                option_bytes.put_u16(port);
            }
            SDOption::Ipv6SDEndpoint { addr, port } => {
                option_bytes.put_u16(0x15);
                option_bytes.push(0x26); //type
                option_bytes.push(0); //reserved
                option_bytes.put_u128(addr.into());
                option_bytes.push(0); //reserved
                option_bytes.push(0x11); //always UDP
                option_bytes.put_u16(port);
            }
        }

        option_bytes
    }
}

#[derive(PartialEq, Debug)]
pub enum ServiceEntryType {
    Find,
    Offer,
}

#[allow(clippy::from_over_into)]
impl Into<u8> for ServiceEntryType {
    fn into(self) -> u8 {
        match self {
            ServiceEntryType::Find => 0,
            ServiceEntryType::Offer => 1,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ServiceEntry {
    data: BitArray<[u8; 16], Msb0>,
}

impl Default for ServiceEntry {
    fn default() -> Self {
        let mut entry = ServiceEntry {
            data: BitArray::default(),
        };
        entry.set_major_version(0xFF);
        entry.set_minor_version(0xFFFFFFFF);
        entry.set_instance_id(0xFFFF);

        entry
    }
}

impl TryFrom<&[u8]> for ServiceEntry {
    type Error = ();

    fn try_from(buf: &[u8]) -> Result<Self, Self::Error> {
        if buf.len() != 16 {
            log::error!("Invalid length for ServiceEntry buffer");
            return Err(());
        }
        let mut data = [0u8; 16];
        for (i, b) in buf.iter().enumerate() {
            data[i] = *b;
        }

        // copy the 16 bytes into the internal buffer. The only
        // validation we do is the type field below
        let mut entry = ServiceEntry {
            data: BitArray::new(data),
        };

        match buf[0] {
            0 => entry.set_service_entry_type(ServiceEntryType::Find),
            1 => entry.set_service_entry_type(ServiceEntryType::Offer),
            _ => return Err(()),
        }

        Ok(entry)
    }
}

impl ServiceEntry {
    pub fn set_service_entry_type(&mut self, ty: ServiceEntryType) {
        let ty: u8 = ty.into();
        self.data[..8].store(ty);
    }

    pub fn service_entry_type(&self) -> Option<ServiceEntryType> {
        let ty: u8 = self.data[..8].load();
        match ty {
            0 => Some(ServiceEntryType::Find),
            1 => Some(ServiceEntryType::Offer),
            _ => None,
        }
    }
    pub fn set_index1_options(&mut self, i: u8, num_options: u8) {
        self.data[8..16].store(i);
        self.data[24..28].store(num_options);
    }

    pub fn index1_options(&self) -> (u8, u8) {
        (self.data[8..16].load(), self.data[24..28].load())
    }

    pub fn set_index2_options(&mut self, i: u8, num_options: u8) {
        self.data[16..24].store(i);
        self.data[28..32].store(num_options);
    }

    pub fn index2_options(&self) -> (u8, u8) {
        (self.data[16..24].load(), self.data[28..32].load())
    }

    pub fn set_service_id(&mut self, service_id: u16) {
        self.data[32..48].store(service_id);
    }

    pub fn service_id(&self) -> u16 {
        self.data[32..48].load()
    }
    pub fn set_instance_id(&mut self, instance_id: u16) {
        self.data[48..64].store(instance_id);
    }

    pub fn instance_id(&self) -> u16 {
        self.data[48..64].load()
    }

    pub fn set_major_version(&mut self, version: u8) {
        self.data[64..72].store(version);
    }

    pub fn major_version(&self) -> u8 {
        self.data[64..72].load()
    }

    pub fn set_ttl(&mut self, ttl: u32) {
        self.data[72..96].store(ttl);
    }

    pub fn ttl(&self) -> u32 {
        self.data[72..96].load()
    }

    pub fn set_minor_version(&mut self, minor_version: u32) {
        self.data[96..].store(minor_version);
    }

    pub fn minor_version(&self) -> u32 {
        self.data[96..].load()
    }

    pub fn as_buffer(&self) -> &[u8] {
        self.data.as_raw_slice()
    }
}

#[derive(Default)]
pub struct EventGroupEntry {
    data: BitArray<[u8; 16], Msb0>,
}

impl EventGroupEntry {
    pub fn set_type(&mut self, ty: ServiceEntryType) {
        let ty: u8 = ty.into();
        self.data[..8].store(ty);
    }
    pub fn set_index_1_options(&mut self, i: u8, num_options: u8) {
        self.data[8..16].store(i);
        self.data[24..28].store(num_options);
    }
    pub fn set_index_2_options(&mut self, i: u8, num_options: u8) {
        self.data[16..24].store(i);
        self.data[28..32].store(num_options);
    }

    pub fn set_initial_data_requested(&mut self, flag: bool) {
        self.data[104..105].store(if flag { 1u8 } else { 0u8 });
    }

    pub fn set_major_version(&mut self, version: u8) {
        self.data[64..72].store(version);
    }

    pub fn set_ttl(&mut self, ttl: u32) {
        self.data[72..96].store(ttl);
    }
    pub fn set_counter(&mut self, counter: u8) {
        self.data[108..112].store(counter);
    }
    pub fn set_event_group_id(&mut self, eventgroup_id: u16) {
        self.data[112..128].store(eventgroup_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]

    fn test_sd_message_basic() {
        let message = SDMessage::default();
        let pkt: SomeIpPacket = message.clone().into();
        let new_msg = SDMessage::try_from(pkt).unwrap();
        assert_eq!(message, new_msg);
    }

    #[test]
    fn test_sd_message() {
        let mut message = SDMessage::default();
        let options = vec![
            SDOption::Ipv4Endpoint {
                addr: Ipv4Addr::new(127, 0, 0, 1),
                transport: SDOptionTransportProto::TCP,
                port: 8090,
            },
            SDOption::Configurations(vec![(
                ConfigurationKey::new("ConfigKey"),
                String::from("ConfigValue"),
            )]),
        ];
        message.set_options(options);
        let mut entry = ServiceEntry::default();
        entry.set_service_entry_type(ServiceEntryType::Offer);
        entry.set_service_id(42);
        message.add_entry(entry, 0, 1, 0, 0).unwrap();
        message.set_exp_initial_data_control(true);
        message.set_reboot(true);

        // the following must fail
        let mut entry = ServiceEntry::default();
        entry.set_service_entry_type(ServiceEntryType::Find);
        entry.set_service_id(0x24);
        if message.add_entry(entry, 0, 2, 0, 0).is_ok() {
            panic!("This should fail");
        }
        // the following must fail
        let mut entry = ServiceEntry::default();
        entry.set_service_entry_type(ServiceEntryType::Find);
        entry.set_service_id(0x24);
        if message.add_entry(entry, 0, 0, 0, 2).is_ok() {
            panic!("This should fail");
        }

        let pkt: SomeIpPacket = message.into();
        let new_entry = SDMessage::try_from(pkt).unwrap();
        assert!(new_entry.is_reboot());
        assert!(new_entry.is_exp_initial_data_control());
        println!("SDMessage:{:?}", new_entry);
    }

    #[test]
    fn test_service_entry() {
        let mut entry = ServiceEntry::default();
        entry.set_service_entry_type(ServiceEntryType::Find);
        assert_eq!(entry.service_entry_type().unwrap(), ServiceEntryType::Find);
        entry.set_index1_options(1, 2);
        assert_eq!((1, 2), entry.index1_options());
        entry.set_index2_options(2, 3);
        assert_eq!((2, 3), entry.index2_options());
        entry.set_instance_id(0x9);
        assert_eq!(0x9, entry.instance_id());
        entry.set_service_id(0x10);
        assert_eq!(0x10, entry.service_id());
        entry.set_major_version(1);
        assert_eq!(1, entry.major_version());
        entry.set_minor_version(0xF0F0F0F0);
        assert_eq!(0xF0F0F0F0, entry.minor_version());
        entry.set_ttl(500);
        assert_eq!(500, entry.ttl());
        assert_eq!(
            entry.as_buffer(),
            &[
                0, 1, 2, 0b00100011, 0b00010000, 0, 0b1001, 0, 1, 0b11110100, 1, 0, 0xF0, 0xF0,
                0xF0, 0xF0
            ]
        );
        let sl = entry.as_buffer();
        let new_entry = ServiceEntry::try_from(&sl[..]).unwrap();
        assert_eq!(entry, new_entry);
    }

    #[test]
    fn test_config_option_configurations() {
        let options = vec![
            (
                ConfigurationKey::new("key1"),
                String::from("ConfigValueOption"),
            ),
            (ConfigurationKey::new("key2"), String::from("value2")),
            (ConfigurationKey::new("key3"), String::from("value3")),
        ];

        let sd_entry = SDOption::Configurations(options);

        let bytes: Vec<u8> = sd_entry.into();
        let serialized = SDOption::try_from(&bytes[..]).expect("Unable to parse into SDOption");
        if let SDOption::Configurations(options) = serialized {
            assert_eq!(options[0].0, ConfigurationKey::new("key1"));
            assert_eq!(options[0].1, String::from("ConfigValueOption"));
            assert_eq!(options[1].0, ConfigurationKey::new("key2"));
            assert_eq!(options[1].1, String::from("value2"));
            assert_eq!(options[2].0, ConfigurationKey::new("key3"));
            assert_eq!(options[2].1, String::from("value3"));
        } else {
            panic!("Expected Configurations");
        }
    }

    #[test]
    fn test_config_option_load_balance() {
        let sd_entry = SDOption::LoadBalancing {
            priority: 5,
            weight: 15,
        };
        let bytes: Vec<u8> = sd_entry.into();
        let serialized = SDOption::try_from(&bytes[..]).expect("Unable to parse into SDOption");
        if let SDOption::LoadBalancing { priority, weight } = serialized {
            assert_eq!(priority, 5);
            assert_eq!(weight, 15);
        } else {
            panic!("Expected LoadBalancing");
        }
    }

    #[test]
    fn test_config_option_ipv4endpoint() {
        let sd_entry = SDOption::Ipv4Endpoint {
            addr: Ipv4Addr::new(127, 0, 0, 1),
            transport: SDOptionTransportProto::TCP,
            port: 8080,
        };
        let bytes: Vec<u8> = sd_entry.into();
        let serialized = SDOption::try_from(&bytes[..]).expect("Unable to parse into SDOption");
        if let SDOption::Ipv4Endpoint {
            addr,
            transport,
            port,
        } = serialized
        {
            assert_eq!(addr, Ipv4Addr::new(127, 0, 0, 1));
            assert_eq!(transport, SDOptionTransportProto::TCP);
            assert_eq!(port, 8080);
        } else {
            panic!("Expected IpV4Endpoint");
        }
    }

    #[test]
    fn test_config_option_ipv6endpoint() {
        let sd_entry = SDOption::Ipv6Endpoint {
            addr: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1),
            transport: SDOptionTransportProto::TCP,
            port: 8080,
        };
        let bytes: Vec<u8> = sd_entry.into();
        let serialized = SDOption::try_from(&bytes[..]).expect("Unable to parse into SDOption");
        if let SDOption::Ipv6Endpoint {
            addr,
            transport,
            port,
        } = serialized
        {
            assert_eq!(addr, Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));
            assert_eq!(transport, SDOptionTransportProto::TCP);
            assert_eq!(port, 8080);
        } else {
            panic!("Expected IpV6Endpoint");
        }
    }

    #[test]
    fn test_config_option_ipv4multicast() {
        let sd_entry = SDOption::Ipv4Multicast {
            addr: Ipv4Addr::new(127, 0, 0, 1),
            port: 8080,
        };
        let bytes: Vec<u8> = sd_entry.into();
        let serialized = SDOption::try_from(&bytes[..]).expect("Unable to parse into SDOption");
        if let SDOption::Ipv4Multicast { addr, port } = serialized {
            assert_eq!(addr, Ipv4Addr::new(127, 0, 0, 1));
            assert_eq!(port, 8080);
        } else {
            panic!("Expected IpV6Endpoint");
        }
    }

    #[test]
    fn test_config_option_ip6multicast() {
        let sd_entry = SDOption::Ipv6Multicast {
            addr: Ipv6Addr::new(127, 0, 0, 1, 0, 0, 0, 0),
            port: 8080,
        };
        let bytes: Vec<u8> = sd_entry.into();
        let serialized = SDOption::try_from(&bytes[..]).expect("Unable to parse into SDOption");
        if let SDOption::Ipv6Multicast { addr, port } = serialized {
            assert_eq!(addr, Ipv6Addr::new(127, 0, 0, 1, 0, 0, 0, 0));
            assert_eq!(port, 8080);
        } else {
            panic!("Expected IpV6Endpoint");
        }
    }

    #[test]
    fn test_config_option_ipv4_sd_endpoint() {
        let sd_entry = SDOption::Ipv4SDEndpoint {
            addr: Ipv4Addr::new(127, 0, 0, 1),
            port: 8080,
        };
        let bytes: Vec<u8> = sd_entry.into();
        let serialized = SDOption::try_from(&bytes[..]).expect("Unable to parse into SDOption");
        if let SDOption::Ipv4SDEndpoint { addr, port } = serialized {
            assert_eq!(addr, Ipv4Addr::new(127, 0, 0, 1));
            assert_eq!(port, 8080);
        } else {
            panic!("Expected IpV4 SD Endpoint");
        }
    }

    #[test]
    fn test_config_option_ipv6_sd_endpoint() {
        let sd_entry = SDOption::Ipv6SDEndpoint {
            addr: Ipv6Addr::new(127, 0, 0, 1, 0, 0, 0, 0),
            port: 8080,
        };
        let bytes: Vec<u8> = sd_entry.into();
        let serialized = SDOption::try_from(&bytes[..]).expect("Unable to parse into SDOption");
        if let SDOption::Ipv6SDEndpoint { addr, port } = serialized {
            assert_eq!(addr, Ipv6Addr::new(127, 0, 0, 1, 0, 0, 0, 0));
            assert_eq!(port, 8080);
        } else {
            panic!("Expected IpV4 SD Endpoint");
        }
    }
}
