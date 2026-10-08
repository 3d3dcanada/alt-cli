use alt_cli::models::{download_verified, import, library};
#[path = "fixtures/gguf.rs"]
mod gguf_fixture;
use sha2::{Digest, Sha256};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

async fn server(
    status: &str,
    headers: String,
    body: Vec<u8>,
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/artifact", listener.local_addr().unwrap());
    let status = status.to_string();
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            socket.read_exact(&mut byte).await.unwrap();
            request.push(byte[0]);
        }
        socket
            .write_all(
                format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{headers}Connection: close\r\n\r\n",
                    body.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        socket.write_all(&body).await.unwrap();
        String::from_utf8(request).unwrap()
    });
    (url, task)
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn cancel() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

#[tokio::test]
async fn interrupted_download_resumes_and_promotes_only_verified_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("model.gguf");
    let bytes = b"GGUFcomplete fixture model bytes";
    std::fs::write(path.with_extension("partial"), &bytes[..9]).unwrap();
    let (url, request) = server(
        "206 Partial Content",
        format!(
            "Content-Range: bytes 9-{}/{}\r\n",
            bytes.len() - 1,
            bytes.len()
        ),
        bytes[9..].to_vec(),
    )
    .await;
    download_verified(
        &url,
        &path,
        bytes.len() as u64,
        &hash(bytes),
        cancel(),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert!(!path.with_extension("partial").exists());
    assert!(
        request
            .await
            .unwrap()
            .to_lowercase()
            .contains("range: bytes=9-")
    );
    // Reusing an already verified artifact needs no live server.
    download_verified(
        "http://127.0.0.1:1/unreachable",
        &path,
        bytes.len() as u64,
        &hash(bytes),
        cancel(),
        |_| {},
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn server_ignoring_range_restarts_instead_of_concatenating() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("model.gguf");
    let bytes = b"GGUFfull content";
    std::fs::write(path.with_extension("partial"), b"GGUFold").unwrap();
    let (url, request) = server("200 OK", String::new(), bytes.to_vec()).await;
    download_verified(
        &url,
        &path,
        bytes.len() as u64,
        &hash(bytes),
        cancel(),
        |_| {},
    )
    .await
    .unwrap();
    request.await.unwrap();
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[tokio::test]
async fn checksum_failure_and_bad_range_never_create_ready_model() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("model.gguf");
    let bytes = b"GGUFcorrupt";
    let (url, request) = server("200 OK", String::new(), bytes.to_vec()).await;
    let error = download_verified(
        &url,
        &path,
        bytes.len() as u64,
        &hash(b"GGUFcorrect"),
        cancel(),
        |_| {},
    )
    .await
    .unwrap_err();
    request.await.unwrap();
    assert!(error.to_string().contains("Checksum mismatch"));
    assert!(!path.exists());
    assert!(path.with_extension("corrupt").exists());
    std::fs::write(path.with_extension("partial"), b"GGUF").unwrap();
    let (url, request) = server(
        "206 Partial Content",
        "Content-Range: bytes 0-6/11\r\n".into(),
        b"garbage".to_vec(),
    )
    .await;
    assert!(
        download_verified(&url, &path, 11, &hash(bytes), cancel(), |_| {})
            .await
            .is_err()
    );
    request.await.unwrap();
    assert_eq!(
        std::fs::read(path.with_extension("partial")).unwrap(),
        b"GGUF"
    );
    assert!(!path.exists());
}

#[tokio::test]
async fn pause_interrupts_a_stalled_body_and_keeps_partial_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("model.gguf");
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (received, wait) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = [0; 1024];
        assert!(stream.read(&mut buf).await.unwrap() > 0);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\n\r\nGGUF")
            .await
            .unwrap();
        received.send(()).unwrap();
        std::future::pending::<()>().await;
    });
    let stop = cancel();
    let stopper = stop.clone();
    tokio::spawn(async move {
        wait.await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        stopper.store(true, Ordering::Relaxed);
    });
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        download_verified(&url, &path, 20, &"a".repeat(64), stop, |_| {}),
    )
    .await
    .unwrap();
    assert!(result.unwrap_err().to_string().contains("paused"));
    assert!(!path.exists());
    assert_eq!(
        std::fs::read(path.with_extension("partial")).unwrap(),
        b"GGUF"
    );
    task.abort();
}

#[tokio::test]
async fn import_preserves_original_and_rejects_wrong_format() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("my model.gguf");
    std::fs::write(&original, gguf_fixture::fixture(None)).unwrap();
    let root = dir.path().join("alt");
    let artifact = import(&root, &original, false, cancel(), |_| {})
        .await
        .unwrap();
    assert_eq!(artifact.path, original.canonicalize().unwrap());
    assert_eq!(library(&root).unwrap().len(), 1);
    assert!(original.exists());
    let wrong = dir.path().join("bad.gguf");
    std::fs::write(&wrong, b"not a model").unwrap();
    assert!(
        import(&root, &wrong, false, cancel(), |_| {})
            .await
            .is_err()
    );
    assert_eq!(library(&root).unwrap().len(), 1);
}

#[tokio::test]
async fn multipart_promotes_only_complete_set_and_relocates_without_import_loss() {
    use alt_cli::models::*;
    use serde_json::json;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("state");
    let a = &gguf_fixture::fixture(Some((0, 2)));
    let b = &gguf_fixture::fixture(Some((1, 2)));
    let catalog_value = json!({"sha":"revision","tags":["heretic"],"siblings":[{"rfilename":"q-00001-of-00002.gguf","size":a.len(),"lfs":{"sha256":hash(a)}},{"rfilename":"q-00002-of-00002.gguf","size":b.len(),"lfs":{"sha256":hash(b)}}]});
    let set = catalog("owner/repo", &catalog_value).unwrap().remove(0);
    assert_eq!(set.parts.len(), 2);
    assert_eq!(set.bytes, (a.len() + b.len()) as u64);
    let mut incomplete = catalog_value.clone();
    incomplete["siblings"].as_array_mut().unwrap().pop();
    assert!(catalog("owner/repo", &incomplete).unwrap().is_empty());
    let (url, request) = server("200 OK", String::new(), a.to_vec()).await;
    assert!(
        download_set(
            &root,
            set.clone(),
            cancel(),
            |_| {},
            |f| Ok(if f.filename.contains("00001-of") {
                url.clone()
            } else {
                "http://127.0.0.1:1/unreachable".into()
            })
        )
        .await
        .is_err()
    );
    request.await.unwrap();
    assert!(library(&root).unwrap().is_empty());
    let (url, request) = server("200 OK", String::new(), b.to_vec()).await;
    let model = download_set(&root, set, cancel(), |_| {}, |_| Ok(url.clone()))
        .await
        .unwrap();
    request.await.unwrap();
    assert_eq!(model.pieces.len(), 2);
    verify_artifact(&model, &cancel(), |_| {}).await.unwrap();
    assert!(
        model
            .path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("00001-of")
    );
    let original = dir.path().join("import.gguf");
    std::fs::write(&original, gguf_fixture::fixture(None)).unwrap();
    let imported = import(&root, &original, true, cancel(), |_| {})
        .await
        .unwrap();
    let moved = dir.path().join("new-cache");
    relocate(&root, &moved, cancel()).await.unwrap();
    let active = artifact(&root, &model.id).unwrap();
    assert!(active.path.starts_with(&moved));
    assert!(model.path.exists());
    verify_artifact(&active, &cancel(), |_| {}).await.unwrap();
    remove(&root, &imported.id, true).unwrap();
    assert_eq!(
        std::fs::read(original).unwrap(),
        gguf_fixture::fixture(None)
    );
    remove(&root, &model.id, true).unwrap();
    assert!(!active.path.exists());
    assert!(model.path.exists());
}

#[test]
fn hub_token_scope_worker() {
    if std::env::var("ALT_TEST_HUB_SCOPE").ok().as_deref() != Some("1") {
        return;
    }
    for (url, allowed) in [
        ("https://huggingface.co/api/models/private/repo", true),
        ("https://huggingface.co:443/api/models", true),
        ("http://huggingface.co/api/models", false),
        ("https://huggingface.co:8443/api/models", false),
        ("https://cdn.huggingface.co/file", false),
        ("https://huggingface.co.example.invalid/file", false),
        ("http://127.0.0.1:1/file", false),
    ] {
        let request = alt_cli::models::hub_request(reqwest::Client::new().get(url), url)
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(
            request
                .headers()
                .get(reqwest::header::AUTHORIZATION)
                .is_some(),
            allowed,
            "{url}"
        );
    }
}
#[test]
fn hub_authentication_is_scoped_without_mutating_the_test_process_environment() {
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "hub_token_scope_worker"])
        .env("ALT_TEST_HUB_SCOPE", "1")
        .env("HF_TOKEN", "fixture-not-a-secret")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[tokio::test]
async fn interrupted_cache_relocation_preserves_originals_and_resumes_same_destination() {
    use alt_cli::models::{artifact, import, relocate};
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let original = outside.path().join("managed.gguf");
    std::fs::write(&original, gguf_fixture::fixture(None)).unwrap();
    let mut model = import(root.path(), &original, true, cancel(), |_| {})
        .await
        .unwrap();
    model.source = Some(alt_cli::models::ModelFile {
        repo: "fixture/test".into(),
        revision: "fixture".into(),
        filename: "managed.gguf".into(),
        bytes: model.bytes,
        sha256: model.sha256.clone(),
        license: "fixture; no weights".into(),
        uncensored_claim: true,
        parts: vec![],
    });
    let record = root
        .path()
        .join("models")
        .join(format!("{}.json", model.id));
    std::fs::write(&record, serde_json::to_vec(&model).unwrap()).unwrap();
    let before = std::fs::read(&record).unwrap();
    let destination = outside.path().join("new-cache");
    let copied = destination.join(&model.id).join("managed.gguf");
    std::fs::create_dir_all(copied.parent().unwrap()).unwrap();
    std::fs::write(&copied, b"conflicting bytes").unwrap();
    assert!(relocate(root.path(), &destination, cancel()).await.is_err());
    assert_eq!(std::fs::read(&record).unwrap(), before);
    assert!(original.exists());
    assert!(root.path().join("cache-relocation.pending.json").is_file());
    assert!(
        relocate(
            root.path(),
            &outside.path().join("wrong-destination"),
            cancel()
        )
        .await
        .is_err()
    );
    std::fs::remove_file(&copied).unwrap();
    relocate(root.path(), &destination, cancel()).await.unwrap();
    assert_eq!(artifact(root.path(), &model.id).unwrap().path, copied);
    assert!(original.exists());
    assert!(!root.path().join("cache-relocation.pending.json").exists());
}
