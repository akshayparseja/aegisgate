#[derive(Debug, PartialEq)]
pub enum MqttPacketType {
    Connect,
    Publish,
    Other,
    Malformed,
}

pub fn inspect_packet(payload: &[u8]) -> MqttPacketType {
    if payload.is_empty() {
        return MqttPacketType::Malformed;
    }

    let packet_type = payload[0] >> 4;

    match packet_type {
        1 => MqttPacketType::Connect,
        3 => MqttPacketType::Publish,
        _ => MqttPacketType::Other,
    }
}
