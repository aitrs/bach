extern crate pnet;
use pnet::packet::icmp::echo_reply::EchoReplyPacket;
use pnet::packet::icmp::echo_request::MutableEchoRequestPacket;
use pnet::packet::icmp::IcmpTypes::EchoRequest;
use pnet::packet::icmp::{checksum, IcmpCode, IcmpPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::MutableIpv4Packet;
use pnet::packet::Packet;
use pnet::transport::icmp_packet_iter;
use pnet::transport::transport_channel;
use pnet::transport::TransportChannelType::Layer3;
use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind};
use std::net::{IpAddr, Ipv4Addr};

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct Host {
    name: String,
    ip: Ipv4Addr,
    port: u16,
    user: String,
    password: String,
}

fn create_icmp_p<'a>(
    ipv4_buffer: &'a mut [u8],
    icmp_buffer: &'a mut [u8],
    dest: Ipv4Addr,
    seq_number: u16,
) -> MutableIpv4Packet<'a> {
    let mut ipv4_packet =
        MutableIpv4Packet::new(ipv4_buffer).expect("Impossible de créer un packet ipv4");
    ipv4_packet.set_version(4);
    ipv4_packet.set_header_length(21);
    ipv4_packet.set_total_length(21 + 8 + 32);
    ipv4_packet.set_next_level_protocol(IpNextHeaderProtocols::Icmp);
    ipv4_packet.set_destination(dest);

    let mut icmp_packet =
        MutableEchoRequestPacket::new(icmp_buffer).expect("Impossible de créer un packet icmp");
    icmp_packet.set_icmp_type(EchoRequest);
    icmp_packet.set_sequence_number(seq_number);
    icmp_packet.set_icmp_code(IcmpCode::new(0));
    let id = std::process::id();
    icmp_packet.set_identifier(id as u16);
    let checksum = checksum(&IcmpPacket::new(icmp_packet.packet()).unwrap());
    icmp_packet.set_checksum(checksum);
    ipv4_packet.set_payload(icmp_packet.packet());

    ipv4_packet
}

impl Host {
    pub fn new(name: &str, ip: Ipv4Addr, user: &str, password: &str) -> Self {
        Host {
            name: name.to_string(),
            ip,
            user: user.to_string(),
            port: 22,
            password: password.to_string(),
        }
    }

    pub fn name(&self) -> String {
        self.name.to_string()
    }

    pub fn user(&self) -> String {
        self.user.to_string()
    }

    pub fn password(&self) -> String {
        self.password.to_string()
    }

    pub fn ip(&self) -> &Ipv4Addr {
        &self.ip
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    pub fn ping_test(&self, times: u16) -> Result<u16, Error> {
        let (mut tx, mut rx) = match transport_channel(
            EchoReplyPacket::minimum_packet_size(),
            Layer3(IpNextHeaderProtocols::Icmp),
        ) {
            Ok((tx, rx)) => (tx, rx),
            Err(e) => return Err(Error::new(ErrorKind::Other, e)),
        };
        let mut ping_count = 0;
        for i in 0..times {
            let mut run = true;
            let mut ip_buffer = [0u8; 61];
            let mut icmp_buffer = [0u8; 31];
            let icmp_packet = create_icmp_p(&mut ip_buffer, &mut icmp_buffer, self.ip, i);
            match tx.send_to(icmp_packet, IpAddr::V4(self.ip)) {
                Ok(_u) => (),
                Err(_e) => {
                    run = false;
                }
            }

            let now = std::time::Instant::now();
            let mut recv_it = icmp_packet_iter(&mut rx);
            while run {
                match recv_it.next_with_timeout(std::time::Duration::from_millis(1000)) {
                    Ok(ret) => if let Some(packet) = ret {
                        if let Some(_reply) = EchoReplyPacket::new(packet.0.packet()) {
                            if packet.1.eq(&IpAddr::V4(self.ip)) {
                                ping_count += 1;
                                run = false;
                            }
                        }                    
                    },
                    Err(e) => return Err(Error::new(ErrorKind::Other, e)),
                }
                if now.elapsed().gt(&std::time::Duration::from_millis(1000)) {
                    run = false;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
        Ok(ping_count)
    }
}
