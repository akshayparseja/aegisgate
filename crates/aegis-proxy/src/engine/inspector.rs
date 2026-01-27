use bytes::BytesMut;

pub enum InspectorResult {
    Valid,
    Invalid(String),
    Incomplete,
}

pub fn inspect_initial_packet(buf: &[u8]) -> InspectorResult {
    if buf.is_empty() {
        return InspectorResult::Incomplete;
    }

    let packet_type = buf[0] >> 4;

    match packet_type {
        1 => InspectorResult::Valid,
        _ => InspectorResult::Invalid(format!("Expected CONNECT (1), got {}", packet_type)),
    }
}
