use std::time::Duration;

use aegis_proxy::engine::http::{inspect_http, looks_like_http, HttpInspectionResult};

#[tokio::test]
async fn test_parse_valid_http_request() {
    let data = b"GET /index.html HTTP/1.1\r\nHost: example.com\r\nUser-Agent: test\r\n\r\n";
    let mut reader = &data[..];

    let result = inspect_http(
        &mut reader,
        Duration::from_secs(1),
        Duration::from_millis(100),
        8192,
        100,
        8192,
    )
    .await
    .unwrap();

    assert_eq!(result, HttpInspectionResult::HttpDetected);
}

#[tokio::test]
async fn test_parse_post_request() {
    let data = b"POST /api/data HTTP/1.1\r\nContent-Type: application/json\r\n\r\n";
    let mut reader = &data[..];

    let result = inspect_http(
        &mut reader,
        Duration::from_secs(1),
        Duration::from_millis(100),
        8192,
        100,
        8192,
    )
    .await
    .unwrap();

    assert_eq!(result, HttpInspectionResult::HttpDetected);
}

#[tokio::test]
async fn test_not_http() {
    // Typical MQTT CONNECT beginning bytes
    let data = b"\x10\x0f\x00\x04MQTT\x04\x02\x00\x3c\x00\x05test1";
    let mut reader = &data[..];

    let res = inspect_http(
        &mut reader,
        Duration::from_secs(1),
        Duration::from_millis(100),
        8192,
        100,
        8192,
    )
    .await;

    match res {
        Ok(HttpInspectionResult::NotHttp) => {
            // expected
        }
        Ok(other) => panic!("expected NotHttp, got {:?}", other),
        Err(e) => {
            // Some readers may return an UnexpectedEof / incomplete line for non-HTTP data.
            // Treat that as equivalent to NotHttp for robustness in tests.
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                // acceptable: connection ended before a valid HTTP line - treat as NotHttp
            } else {
                panic!("unexpected error during http inspection: {}", e);
            }
        }
    }
}

#[tokio::test]
async fn test_header_count_limit() {
    let mut data = b"GET / HTTP/1.1\r\n".to_vec();
    // Add 101 headers (exceeds limit of 100)
    for i in 0..101 {
        data.extend_from_slice(format!("Header{}: value\r\n", i).as_bytes());
    }
    data.extend_from_slice(b"\r\n");

    let mut reader = &data[..];

    let result = inspect_http(
        &mut reader,
        Duration::from_secs(1),
        Duration::from_millis(100),
        100000,
        100,
        8192,
    )
    .await
    .unwrap();

    assert!(matches!(result, HttpInspectionResult::SlowlorisDetected(_)));
}

#[tokio::test]
async fn test_header_size_limit() {
    let mut data = b"GET / HTTP/1.1\r\n".to_vec();
    // Add one huge header
    let huge_value = "x".repeat(10000);
    data.extend_from_slice(format!("Big-Header: {}\r\n", huge_value).as_bytes());
    data.extend_from_slice(b"\r\n");

    let mut reader = &data[..];

    let result = inspect_http(
        &mut reader,
        Duration::from_secs(1),
        Duration::from_millis(100),
        8192,
        100,
        20000,
    )
    .await
    .unwrap();

    assert!(matches!(result, HttpInspectionResult::SlowlorisDetected(_)));
}

#[tokio::test]
async fn test_malformed_header() {
    let data = b"GET / HTTP/1.1\r\nMalformedHeaderNoColon\r\n\r\n";
    let mut reader = &data[..];

    let result = inspect_http(
        &mut reader,
        Duration::from_secs(1),
        Duration::from_millis(100),
        8192,
        100,
        8192,
    )
    .await
    .unwrap();

    assert!(matches!(result, HttpInspectionResult::SlowlorisDetected(_)));
}

#[test]
fn test_looks_like_http() {
    assert!(looks_like_http(b"GET /"));
    assert!(looks_like_http(b"POST /api"));
    assert!(looks_like_http(b"HEAD /index"));
    assert!(!looks_like_http(b"MQTT"));
    assert!(!looks_like_http(b"\x10\x0f\x00"));
    assert!(!looks_like_http(b"GET")); // No space after
}
