
use std::net::{Ipv4Addr, Ipv6Addr};
pub enum SDOptionTransportProto {
    UDP,
    TCP,
}

pub struct ConfigurationKey(String);

#[rustfmt::skip]
pub enum SDOption {
    Configuration{ key:ConfigurationKey, value:String},
    LoadBalancing { priority: u16, weight: u16 },
    Ipv4Endpoint { addr : Ipv4Addr , transport : SDOptionTransportProto, port: u16},
    Ipv6Endpoint { addr : Ipv6Addr , transport : SDOptionTransportProto, port: u16},
    Ipv4Multicast { addr: Ipv4Addr, port: u16},
    Ipv6Multicast { addr: Ipv6Addr, port: u16},
    Ipv4SDEndpoint {addr : Ipv4Addr, port: u16},
    Ipv6SDEndpoint {addr : Ipv6Addr, port: u16},

}

pub struct ServiceEntry {
    entry_type: u8,
    option_first: u8,
    option_second: u8,
    num_option1: u8,
    num_option2: u8,
}
