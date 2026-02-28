use std::time::Duration;

use aegis_proxy::engine::slowloris::{read_with_idle_timeout, read_with_timeout, TimeoutReader};
use tokio::io::AsyncReadExt;

#[tokio::test]
async fn test_read_with_timeout_success() {
    let data = b"hello world";
    let mut reader = &data[..];
    let mut buf = vec![0u8; 11];

    let result = read_with_timeout(&mut reader, &mut buf, Duration::from_secs(1)).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 11);
    assert_eq!(&buf, data);
}

#[tokio::test]
async fn test_timeout_reader_wrapper_reads_entire_buffer() {
    let data = b"test data";
    let reader = &data[..];
    let mut timeout_reader = TimeoutReader::new(reader, Duration::from_secs(1));

    let mut buf = vec![0u8; 9];
    // Need AsyncReadExt in scope for `.read(...)`
    let n = timeout_reader.read(&mut buf).await.unwrap();
    assert_eq!(n, 9);
    assert_eq!(&buf, data);
}

#[tokio::test]
async fn test_read_with_idle_timeout_success() {
    // Ensure read_with_idle_timeout reads up to the provided buffer length
    let data = b"abcdefghijklmnopqrstuvwxyz";
    let mut reader = &data[..];
    let mut buf = vec![0u8; 10];

    let result = read_with_idle_timeout(
        &mut reader,
        &mut buf,
        Duration::from_secs(1), // per-byte idle timeout
        Duration::from_secs(5), // total timeout
    )
    .await;

    assert!(result.is_ok());
    let n = result.unwrap();
    assert_eq!(n, 10);
    assert_eq!(&buf, &data[..10]);
}
