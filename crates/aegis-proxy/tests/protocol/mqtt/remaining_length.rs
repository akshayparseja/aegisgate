use aegis_proxy::parser::mqtt::decode_remaining_length;

#[test]
fn rl_decodes_zero_single_byte() {
    let buf = [0x00u8];
    let res = decode_remaining_length(&buf).unwrap();
    assert_eq!(res, (0usize, 1usize));
}

#[test]
fn rl_decodes_max_single_byte_127() {
    let buf = [0x7Fu8];
    let res = decode_remaining_length(&buf).unwrap();
    assert_eq!(res, (127usize, 1usize));
}

#[test]
fn rl_decodes_two_bytes_128() {
    // 128 => 0x80 0x01
    let buf = [0x80u8, 0x01u8];
    let res = decode_remaining_length(&buf).unwrap();
    assert_eq!(res, (128usize, 2usize));
}

#[test]
fn rl_decodes_multi_byte_example_321() {
    // 321 => 0x41 0x02 (65 + 2*128)
    let buf = [0x41u8, 0x02u8];
    let res = decode_remaining_length(&buf).unwrap();
    assert_eq!(res, (321usize, 2usize));
}

#[test]
fn rl_decodes_max_allowed_value() {
    // Max per MQTT spec: 268_435_455 => 0xFF 0xFF 0xFF 0x7F
    let buf = [0xFFu8, 0xFFu8, 0xFFu8, 0x7Fu8];
    let res = decode_remaining_length(&buf).unwrap();
    assert_eq!(res, (268435455usize, 4usize));
}

#[test]
fn rl_incomplete_returns_error() {
    // Continuation bit set, but no following byte -> incomplete
    let buf = [0x81u8];
    let err = decode_remaining_length(&buf).unwrap_err();
    assert_eq!(err, "Incomplete");
}

#[test]
fn rl_malformed_exceeds_four_bytes() {
    // Four continuation bytes and no terminating byte => malformed
    let buf = [0x80u8, 0x80u8, 0x80u8, 0x80u8];
    let res = decode_remaining_length(&buf);
    assert!(res.is_err());
    // The implementation may return "Malformed" or another error string; ensure it's treated as error.
}
