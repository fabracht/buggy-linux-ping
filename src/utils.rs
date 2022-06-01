use std::time::{Duration, Instant};

use log::{error, warn};
use pnet::packet::icmp::{destination_unreachable, echo_reply, echo_request, time_exceeded};

pub enum IcmpPacketType<'p> {
    EchoReply(echo_reply::EchoReplyPacket<'p>),
    EchoRequest(echo_request::EchoRequestPacket<'p>),
    TimeExceeded(time_exceeded::TimeExceededPacket<'p>),
    DestinationUnreachable(destination_unreachable::DestinationUnreachablePacket<'p>),
    Err(u8),
}

pub fn calculate_duration(
    parsed_packet: IcmpPacketType,
    reply_packet_seq_number: &mut u16,
    bucket: &[Instant],
    delta_t: &mut Duration,
    received_at: Instant,
) -> u8 {
    let packet_type = match parsed_packet {
        // Echo reply is the only one that contributes to the timing statistics?
        IcmpPacketType::EchoReply(reply) => {
            // Retrieve seq_number from reply
            *reply_packet_seq_number = reply.get_sequence_number();
            // This index is guaranteed to exist
            let index = *reply_packet_seq_number as usize;

            ///////////////////////////////////////////////////
            // Calculate RTT
            ///////////////////////////////////////////////
            let sent_at = bucket[index];
            *delta_t = received_at.duration_since(sent_at);
            // let delta_t_micros = delta_t.as_secs_f64() * 1000.0; // Multiply seconds by 1000 gives you milliseconds
            reply.get_icmp_type().0
        }
        IcmpPacketType::EchoRequest(request) => {
            // DO NOTHING: Doesn't apply in this case, unless we want to be able in the future of responding to ping requests;
            request.get_icmp_type().0
        }
        IcmpPacketType::TimeExceeded(exceeded) => {
            warn!("TimeExceeded");
            *reply_packet_seq_number = 0;
            exceeded.get_icmp_type().0
        }
        IcmpPacketType::DestinationUnreachable(unreachable) => {
            warn!("DestinationUnreachable");
            *reply_packet_seq_number = 0;
            unreachable.get_icmp_type().0
        }
        IcmpPacketType::Err(packet_type) => {
            error!("Packet type: {}", packet_type);
            *reply_packet_seq_number = 0;
            packet_type
        }
    };
    packet_type
}
