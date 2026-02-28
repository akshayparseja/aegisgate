#[derive(Debug, PartialEq)]
pub enum MqttPacketType {
    Connect,
    Publish,
    Other,
    Malformed,
}

/// Decode the MQTT Remaining Length per the MQTT spec.
///
/// Returns Ok((value, bytes_used)) on success, or Err(&'static str) on error:
/// - "Incomplete": buffer ended before the Remaining Length finished
/// - "Malformed": encoding uses more than 4 bytes (protocol error)
pub fn decode_remaining_length(buf: &[u8]) -> Result<(usize, usize), &'static str> {
    let mut multiplier: usize = 1;
    let mut value: usize = 0;
    let mut bytes_used: usize = 0;

    for &b in buf.iter() {
        bytes_used += 1;
        let digit = (b & 0x7F) as usize;
        value += digit * multiplier;

        // If this byte has MSB == 0, it's the last byte
        if (b & 0x80) == 0 {
            return Ok((value, bytes_used));
        }

        multiplier = multiplier.checked_mul(128).ok_or("Malformed")?;

        // MQTT spec limits remaining length encoding to at most 4 bytes
        if bytes_used >= 4 && (b & 0x80) != 0 {
            return Err("Malformed");
        }
    }

    // We ran out of input before the remaining-length completed
    Err("Incomplete")
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
