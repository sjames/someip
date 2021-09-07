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

use std::net::{IpAddr, Ipv4Addr};

#[derive(Clone)]
pub struct Configuration {
    pub initial_delay_min: std::time::Duration,
    pub initial_delay_max: std::time::Duration,
    pub repetition_base_delay: std::time::Duration,
    pub repetitions_max: u32,
    pub request_response_delay: std::time::Duration,
    pub cyclic_offer_delay: std::time::Duration,
    pub sd_port: u16,
    pub sd_multicast_ip: IpAddr,
    pub max_packet_size_tcp: u32,
    pub reconnection_delay: std::time::Duration,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            initial_delay_min: std::time::Duration::from_millis(10),
            initial_delay_max: std::time::Duration::from_millis(10),
            repetition_base_delay: std::time::Duration::from_millis(10),
            repetitions_max: 10,
            request_response_delay: std::time::Duration::from_millis(10),
            cyclic_offer_delay: std::time::Duration::from_millis(10),
            sd_port: crate::someip_codec::SD_PORT,
            sd_multicast_ip: IpAddr::V4("224.0.2.1".parse::<Ipv4Addr>().unwrap()),
            max_packet_size_tcp: 1024 * 4, // 4KiB default
            reconnection_delay: std::time::Duration::from_millis(500),
        }
    }
}
