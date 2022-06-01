use std::{
    collections::HashMap,
    net::IpAddr,
    time::{Duration, Instant},
};

use crate::{
    config::Configuration,
    results::PingResults,
    utils::{calculate_duration, IcmpPacketType},
};
use log::{debug, error};
use pnet::{
    packet::{
        icmp::{
            destination_unreachable, echo_reply,
            echo_request::{self, MutableEchoRequestPacket},
            time_exceeded, IcmpPacket, IcmpTypes,
        },
        MutablePacket, Packet,
    },
    transport::TransportChannelType::Layer3,
};
use pnet::{
    packet::{ip::IpNextHeaderProtocols, ipv4::MutableIpv4Packet},
    transport::ipv4_packet_iter,
};

static IPV4_HEADER_LEN: usize = 21;
static IPV4_BUFFER: usize = 1024;
static ICMP_HEADER_LEN: usize = 8;

pub fn run_ping(configuration: &Configuration) -> PingResults {
    // Collect the results from the statistics plus some other useful information
    let mut ping_results = PingResults::default();

    // Set the counter for the main loop
    let mut initial_ping_count = 1;
    // Initialize on packet 1 (ICMP packet seq numbers start at 1, max 255)
    let mut seq_number = 1;
    // let ttl = configuration.ttl as u64;

    let bucket_size = (configuration.number_of_pings + 1) as usize;
    let mut bucket = vec![Instant::now(); bucket_size];
    let mut result_map: HashMap<usize, (u128, u8)> = HashMap::with_capacity(bucket_size);
    let mut reply_packet_seq_number: u16 = 1;

    let mut buffer_ip = vec![0u8; IPV4_HEADER_LEN + IPV4_BUFFER];
    let mut buffer_icmp = vec![0u8; configuration.payload_size + ICMP_HEADER_LEN]; // 8 header bytes, then payload

    let dest: IpAddr = configuration
        .host
        .parse()
        .expect("Failed to parse host address");

    let start = Instant::now();
    let ping_duration_from_interval = Duration::from_micros(configuration.ping_interval);
    let (mut transport_sender, mut transport_receiver) = pnet::transport::transport_channel(
        4096,
        Layer3(IpNextHeaderProtocols::Icmp),
    )
    .map_err(|e| e.to_string())
    .expect(
        "Failed creating transport channel. Try to `sudo setcap cap_net_raw+ep /path/to/exec`.\
         Or run program with sudo.",
    );
    // Create transport iterator
    let mut iter = ipv4_packet_iter(&mut transport_receiver);
    'main: loop {
        // Start the spent_time cost
        let _spent_time_start = Instant::now();
        if initial_ping_count <= configuration.number_of_pings {
            ///////////////////////////////////
            // Send Message and decrement the ping counter
            ///////////////////////////////////
            let ipv4_packet = create_ip_packet(
                &mut buffer_ip,
                configuration,
                &mut buffer_icmp,
                initial_ping_count,
            );
            debug!("Sending echo request {}", seq_number);

            // Set the Instant the message was sent
            let now = Instant::now();
            bucket[seq_number as usize] = now;

            // Send the message
            match transport_sender.send_to(ipv4_packet, dest) {
                Ok(_sent) => {
                    debug!("Sent icmp packet to {} with icmp_seq {}", dest, seq_number);
                    seq_number += 1;
                }
                Err(e) => error!(
                    "Failed sending packet to {}  with icmp_seq {} : {}",
                    dest, 1, e
                ),
            }
            initial_ping_count += 1;
        }

        ////////////////////////////////////////////////
        // Timeout Bug:
        //////////////////////////////////////////////
        while let Ok(option) = iter.next_with_timeout(Duration::from_millis(100)) {
            let received_at = Instant::now();
            // debug!("Received message {:?} at {:?}", option, received_at);

            if let Some((packet, _addr)) = option {
                // Initialize time variables
                let mut delta_t = Duration::new(0, 0);

                // Check if dscp changed
                let dscp = packet.get_dscp();
                if dscp != configuration.dscp {
                    ping_results.init_dscp_return = dscp;
                    ping_results.dscp_changed = true;
                }

                // Leaving this for convenience when implementing IPv6
                let source = packet.get_source();
                let destination = packet.get_destination();

                // Handle incoming packet
                let parsed_packet = handle_icmp_packet(
                    IpAddr::V4(source),
                    IpAddr::V4(destination),
                    packet.payload(),
                );

                // Calculate RTT and retrieve packet number
                let packet_type = calculate_duration(
                    parsed_packet,
                    &mut reply_packet_seq_number,
                    &bucket,
                    &mut delta_t,
                    received_at,
                );

                debug!("Received message {}", reply_packet_seq_number);

                // If
                if reply_packet_seq_number != 0 {
                    result_map
                        .entry(reply_packet_seq_number as usize)
                        .or_insert((delta_t.as_micros(), packet_type));
                }
            } else {
                break;
            }
        }

        if Instant::now().duration_since(start) > configuration.duration {
            debug!("Breaking main");
            break 'main;
        }
    }

    //////////////////////////////////////////
    // This is where we collect the results
    //////////////////////////////////////////
    ping_results.rtt_statistics.replies = result_map;
    ping_results
}

fn create_ip_packet<'a>(
    buffer_ip: &'a mut [u8],
    configuration: &'a Configuration,
    buffer_icmp: &'a mut Vec<u8>,
    initial_ping_count: u8,
) -> MutableIpv4Packet<'a> {
    // Create Ip packet
    let mut ipv4_packet = MutableIpv4Packet::new(buffer_ip).expect("Error creating ipv4 packet");
    ipv4_packet.set_version(4);
    ipv4_packet.set_header_length(IPV4_HEADER_LEN as u8);
    let total_length = (IPV4_HEADER_LEN + ICMP_HEADER_LEN + configuration.payload_size) as u16;
    ipv4_packet.set_total_length(total_length);
    ipv4_packet.set_ttl(configuration.ttl);
    ipv4_packet.set_next_level_protocol(IpNextHeaderProtocols::Icmp);
    ipv4_packet.set_destination(configuration.host.parse().unwrap());
    ipv4_packet.set_source(configuration.source_ip.parse().unwrap());
    ipv4_packet.set_identification(13);
    let mut icmp_packet =
        MutableEchoRequestPacket::new(buffer_icmp).expect("Error creating icmp packet");
    icmp_packet.set_identifier(10);
    icmp_packet.set_sequence_number(initial_ping_count.into());
    icmp_packet.set_icmp_type(IcmpTypes::EchoRequest);
    let checksum = pnet::util::checksum(icmp_packet.packet_mut(), 1);
    icmp_packet.set_checksum(checksum);
    ipv4_packet.set_payload(icmp_packet.packet_mut());
    ipv4_packet
}

pub fn handle_icmp_packet(source: IpAddr, destination: IpAddr, packet: &'_ [u8]) -> IcmpPacketType {
    let icmp_packet = IcmpPacket::new(packet);

    if let Some(icmp_packet) = icmp_packet {
        match icmp_packet.get_icmp_type() {
            IcmpTypes::EchoReply => {
                let echo_reply_packet = echo_reply::EchoReplyPacket::new(packet).unwrap();
                let _seq_number = echo_reply_packet.get_sequence_number();
                IcmpPacketType::EchoReply(echo_reply_packet)
            }
            IcmpTypes::EchoRequest => {
                let echo_request_packet = echo_request::EchoRequestPacket::new(packet).unwrap();
                IcmpPacketType::EchoRequest(echo_request_packet)
            }
            IcmpTypes::TimeExceeded => {
                let time_exceeded_packet = time_exceeded::TimeExceededPacket::new(packet).unwrap();
                IcmpPacketType::TimeExceeded(time_exceeded_packet)
            }
            IcmpTypes::DestinationUnreachable => {
                let dest_unreachable =
                    destination_unreachable::DestinationUnreachablePacket::new(packet).unwrap();
                IcmpPacketType::DestinationUnreachable(dest_unreachable)
            }
            _ => {
                error!(
                    "ICMP packet {} -> {} (type={:?})",
                    source,
                    destination,
                    icmp_packet.get_icmp_type()
                );
                IcmpPacketType::Err(0)
            }
        }
    } else {
        error!("Malformed ICMP Packet");
        IcmpPacketType::Err(0)
    }
}
