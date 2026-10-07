//! Real HTTP truncation over repeated multi-megabyte resumptions.
use alt_cli::models::download_verified;
use sha2::{Digest, Sha256};
use std::sync::{Arc, atomic::AtomicBool};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[tokio::test]
async fn repeated_network_loss_preserves_offsets_and_only_promotes_the_complete_hash() {
    let bytes = Arc::new(
        (0..8 * 1024 * 1024)
            .map(|i| (i % 251) as u8)
            .collect::<Vec<_>>(),
    );
    let expected = format!("{:x}", Sha256::digest(&*bytes));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/fixture", listener.local_addr().unwrap());
    let source = bytes.clone();
    let server = tokio::spawn(async move {
        let mut offsets = Vec::new();
        for _ in 0..20 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut b = [0];
                stream.read_exact(&mut b).await.unwrap();
                request.push(b[0]);
            }
            let request = String::from_utf8(request).unwrap().to_lowercase();
            let offset: usize = request
                .lines()
                .find_map(|line| {
                    line.strip_prefix("range: bytes=")
                        .and_then(|n| n.trim_end_matches('-').parse().ok())
                })
                .unwrap_or(0);
            offsets.push(offset);
            let status = if offset == 0 {
                "200 OK"
            } else {
                "206 Partial Content"
            };
            let range = if offset == 0 {
                String::new()
            } else {
                format!(
                    "Content-Range: bytes {offset}-{}/{}\r\n",
                    source.len() - 1,
                    source.len()
                )
            };
            stream.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\n{range}Connection: close\r\n\r\n",source.len()-offset).as_bytes()).await.unwrap();
            // A write can be partially received before disconnection. Resume
            // must use actual durable bytes, not an assumed network chunk size.
            stream
                .write_all(&source[offset..(offset + 1024 * 1024).min(source.len())])
                .await
                .unwrap();
            if offset + 1024 * 1024 >= source.len() {
                break;
            }
        }
        offsets
    });
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("fixture.gguf");
    let mut expected_offsets = vec![0usize];
    for _ in 0..20 {
        let result = download_verified(
            &url,
            &target,
            bytes.len() as u64,
            &expected,
            Arc::new(AtomicBool::new(false)),
            |_| {},
        )
        .await;
        if result.is_err() {
            assert!(!target.exists());
            let offset = target.with_extension("partial").metadata().unwrap().len() as usize;
            assert!(offset > *expected_offsets.last().unwrap());
            assert!(offset < bytes.len());
            expected_offsets.push(offset);
        } else {
            break;
        }
    }
    assert_eq!(server.await.unwrap(), expected_offsets);
    assert_eq!(std::fs::read(target).unwrap(), *bytes);
}
