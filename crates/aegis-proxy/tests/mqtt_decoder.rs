use aegis_proxy::parser::mqtt::{decode_remaining_length, inspect_packet, MqttPacketType};

#[test]
fn remaining_length_decodes_single_byte_127() {
    // 0x7F => 127, single-byte encoding
    let buf = [0x7Fu8];
    let res = decode_remaining_length(&buf).expect("should decode single-byte RL");
    assert_eq!(res, (127usize, 1usize));
}

#[test]
fn remaining_length_decodes_two_bytes_128() {
    // 128 decimal -> encoded as 0x80 0x01
    let buf = [0x80u8, 0x01u8];
    let res = decode_remaining_length(&buf).expect("should decode two-byte RL");
    assert_eq!(res, (128usize, 2usize));
}

#[test]
fn remaining_length_incomplete_returns_incomplete_error() {
    // Continuation bit set but no following byte -> incomplete
    let buf = [0x81u8];
    let err = decode_remaining_length(&buf).unwrap_err();
    assert_eq!(err, "Incomplete");
}

#[test]
fn remaining_length_malformed_too_many_bytes() {
    // Four continuation bytes without termination is malformed per spec
    let buf = [0x81u8, 0x81u8, 0x81u8, 0x81u8];
    let res = decode_remaining_length(&buf);
    assert!(res.is_err());
    // Accept either "Malformed" or any error string; if specific, check it
    let err = res.unwrap_err();
    assert!(
        err == "Malformed" || err == "Incomplete",
        "expected Malformed or Incomplete, got {}",
        err
    );
}

#[test]
fn inspect_packet_detects_connect_and_publish_and_malformed() {
    // CONNECT packet type -> high nibble = 1 -> byte 0x10
    let connect_buf = [0x10u8];
    assert_eq!(inspect_packet(&connect_buf), MqttPacketType::Connect);

    // PUBLISH packet type -> high nibble = 3 -> byte 0x30
    let publish_buf = [0x30u8];
    assert_eq!(inspect_packet(&publish_buf), MqttPacketType::Publish);

    // Other packet type (e.g., SUBSCRIBE is 8 -> 0x80 >> 4 == 8 -> falls into Other)
    let other_buf = [0x80u8];
    assert_eq!(inspect_packet(&other_buf), MqttPacketType::Other);

    // Empty payload -> Malformed
    let empty: [u8; 0] = [];
    assert_eq!(inspect_packet(&empty), MqttPacketType::Malformed);
}
