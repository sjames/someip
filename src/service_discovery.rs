use bincode::config;
use bitvec::prelude::*;
use byteorder::{BigEndian, ByteOrder};
use bytes::BufMut;
use std::{
    convert::TryFrom,
    fmt::Display,
    net::{Ipv4Addr, Ipv6Addr},
    option,
};

#[derive(Debug, PartialEq)]
pub enum SDOptionTransportProto {
    UDP,
    TCP,
}
#[derive(Debug, PartialEq)]
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

#[derive(Debug,PartialEq)]
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
        let length: usize = BigEndian::read_u16(&data[0..2]) as usize;
        if data.len() != length + 3 {
            println!("Invalid length in packet");
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
                    let priority = BigEndian::read_u16(&data[4..6]);
                    let weight = BigEndian::read_u16(&data[6..8]);
                    Ok(SDOption::LoadBalancing { priority, weight })
                }
            }
            0x04 => {
                // IpV4Endpoint
                if length != 9 {
                    Err(())
                } else {
                    let address = BigEndian::read_u32(&data[4..8]);
                    let l4_proto = match data[9] {
                        0x6 => SDOptionTransportProto::TCP,
                        0x11 => SDOptionTransportProto::UDP,
                        _ => return Err(()),
                    };
                    let port = BigEndian::read_u16(&data[10..12]);
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
                    let address = BigEndian::read_u128(&data[4..]);
                    let l4_proto = match data[21] {
                        0x6 => SDOptionTransportProto::TCP,
                        0x11 => SDOptionTransportProto::UDP,
                        _ => return Err(()),
                    };
                    let port = BigEndian::read_u16(&data[22..24]);
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
                    let address = BigEndian::read_u32(&data[4..8]);
                    let _l4_proto = match data[9] {
                        0x11 => SDOptionTransportProto::UDP, // always UDP
                        _ => return Err(()),
                    };
                    let port = BigEndian::read_u16(&data[10..12]);
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
                    let address = BigEndian::read_u128(&data[4..]);
                    let _l4_proto = match data[21] {
                        0x11 => SDOptionTransportProto::UDP,
                        _ => return Err(()),
                    };
                    let port = BigEndian::read_u16(&data[22..24]);
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
                    let address = BigEndian::read_u32(&data[4..8]);
                    let _l4_proto = match data[9] {
                        0x11 => SDOptionTransportProto::UDP, // always UDP
                        _ => return Err(()),
                    };
                    let port = BigEndian::read_u16(&data[10..12]);
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
                    let address = BigEndian::read_u128(&data[4..]);
                    let _l4_proto = match data[21] {
                        0x11 => SDOptionTransportProto::UDP,
                        _ => return Err(()),
                    };
                    let port = BigEndian::read_u16(&data[22..24]);
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
                let mut buf = [0u8; 2];
                byteorder::BigEndian::write_u16(&mut buf, config_option_len);

                //let mut config_option_bytes = Vec::new();
                option_bytes.extend(buf);
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

pub enum ServiceEntryType {
    Find,
    Offer,
}

impl Into<u8> for ServiceEntryType {
    fn into(self) -> u8 {
        match self {
            ServiceEntryType::Find => 0,
            ServiceEntryType::Offer => 1,
        }
    }
}

#[derive(Debug)]
pub struct ServiceEntry {
    data: BitArray<Msb0, [u8; 16]>,
}

impl Default for ServiceEntry {
    fn default() -> Self {
        ServiceEntry {
            data: BitArray::default(),
        }
    }
}

impl ServiceEntry {
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
    pub fn set_service_id(&mut self, service_id: u16) {
        self.data[32..48].store(service_id);
    }
    pub fn set_instance_id(&mut self, instance_id: u16) {
        self.data[48..64].store(instance_id);
    }
    pub fn set_major_version(&mut self, version: u8) {
        self.data[64..72].store(version);
    }

    pub fn set_ttl(&mut self, ttl: u32) {
        self.data[72..96].store(ttl);
    }

    pub fn set_minor_version(&mut self, minor_version: u32) {
        self.data[96..].store(minor_version);
    }

    pub fn as_buffer(&self) -> &[u8; 16] {
        self.data.as_buffer()
    }
}

pub struct EventGroupEntry {
    data: BitArray<Msb0, [u8; 16]>,
}

impl Default for EventGroupEntry {
    fn default() -> Self {
        EventGroupEntry {
            data: BitArray::default(),
        }
    }
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
    use std::convert::TryInto;

    use super::*;

    #[test]
    fn test_service_entry() {
        let mut entry = ServiceEntry::default();
        entry.set_type(ServiceEntryType::Find);
        entry.set_index_1_options(1, 2);
        entry.set_index_2_options(2, 3);
        entry.set_instance_id(0x9);
        entry.set_service_id(0x10);
        entry.set_major_version(1);
        entry.set_minor_version(0xF0F0F0F0);
        entry.set_ttl(500);
        assert_eq!(
            entry.as_buffer(),
            &[
                0, 1, 2, 0b00100011, 0b00010000, 0, 0b1001, 0, 1, 0b11110100, 1, 0, 0xF0, 0xF0,
                0xF0, 0xF0
            ]
        );
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
