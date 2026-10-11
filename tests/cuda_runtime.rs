//! A08: pinned CUDA runtime archive: download, verify, unpack and launch argv/env.
//! Fixtures only; no GPU, no CUDA libraries, no model weights.
use alt_cli::{
    config::{Preferences, Profile, Provider},
    models,
    runtime::{self, Pin},
};
use std::sync::{Arc, atomic::AtomicBool};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn archive() -> Vec<u8> {
    let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
        Vec::new(),
        flate2::Compression::fast(),
    ));
    let script = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/fake_llama_server.py"
    ))
    .unwrap();
    for (name, bytes, mode) in [
        (
            "llama-cuda12-fixture/llama-server",
            script.as_slice(),
            0o755,
        ),
        (
            "llama-cuda12-fixture/libggml-cuda.so",
            b"not a real library".as_slice(),
            0o644,
        ),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(mode);
        header.set_cksum();
        builder.append_data(&mut header, name, bytes).unwrap();
    }
    builder.into_inner().unwrap().finish().unwrap()
}

async fn serve(bytes: Vec<u8>) -> String {
    let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "http://{}/llama-cuda12-fixture.tar.gz",
        socket.local_addr().unwrap()
    );
    tokio::spawn(async move {
        while let Ok((mut connection, _)) = socket.accept().await {
            let mut head = [0u8; 4096];
            let _ = connection.read(&mut head).await;
            let _ = connection
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        bytes.len()
                    )
                    .as_bytes(),
                )
                .await;
            let _ = connection.write_all(&bytes).await;
        }
    });
    url
}

#[tokio::test]
async fn pinned_cuda_archive_is_verified_installed_and_launched_with_its_libraries() {
    let root = tempfile::tempdir().unwrap();
    let bytes = archive();
    let sha = alt_cli::project::digest(&bytes);
    let url: &'static str = Box::leak(serve(bytes.clone()).await.into_boxed_str());
    let sha: &'static str = Box::leak(sha.into_boxed_str());
    // A wrong checksum installs nothing.
    let wrong = Pin {
        label: "fixture CUDA runtime",
        url,
        sha256: "0000000000000000000000000000000000000000000000000000000000000000",
        size: bytes.len() as u64,
        directory: "llama-cuda12-fixture",
    };
    assert!(
        runtime::install_pin(
            root.path(),
            &wrong,
            Arc::new(AtomicBool::new(false)),
            |_| {}
        )
        .await
        .is_err()
    );
    assert!(!root.path().join("tools/llama-cuda12-fixture").exists());
    let pin = Pin {
        sha256: sha,
        ..wrong
    };
    let server = runtime::install_pin(root.path(), &pin, Arc::new(AtomicBool::new(false)), |_| {})
        .await
        .unwrap();
    assert_eq!(
        server,
        root.path().join("tools/llama-cuda12-fixture/llama-server")
    );
    assert!(
        root.path()
            .join("tools/llama-cuda12-fixture/libggml-cuda.so")
            .is_file()
    );
    // Reinstalling replaces the previous copy in place.
    runtime::install_pin(root.path(), &pin, Arc::new(AtomicBool::new(false)), |_| {})
        .await
        .unwrap();

    // Launch: GPU layers as chosen, libraries beside the server on LD_LIBRARY_PATH.
    let original = root.path().join("fixture.gguf");
    let mut gguf = b"GGUF".to_vec();
    gguf.extend(3u32.to_le_bytes());
    gguf.extend([0u8; 16]);
    std::fs::write(&original, gguf).unwrap();
    let model = models::import(
        root.path(),
        &original,
        true,
        Arc::new(AtomicBool::new(false)),
        |_| {},
    )
    .await
    .unwrap();
    let argv_file = root.path().join("argv.json");
    let env_file = root.path().join("env.json");
    // SAFETY: this test binary contains this single test; children read the variables.
    unsafe {
        std::env::set_var("ALT_FAKE_ARGV", &argv_file);
        std::env::set_var("ALT_FAKE_ENV", &env_file);
        std::env::set_var("ALT_FAKE_GPU", "1");
    }
    let mut preferences = Preferences {
        runtime_path: Some(server.clone()),
        ..Preferences::default()
    };
    preferences.runtime.gpu_layers = 99;
    let profile = Profile {
        provider: Provider::Openai,
        endpoint: "http://127.0.0.1:1/v1".into(),
        model: "fixture-alias".into(),
        context_tokens: 4096,
        max_turns: 2,
        uncensored: true,
        api_key_env: None,
        local_model: Some(model.id),
        inference: None,
        runtime_key: None,
    };
    let mut owned = runtime::LocalRuntime::start(
        root.path(),
        &preferences,
        &profile,
        Arc::new(AtomicBool::new(false)),
        |_| {},
    )
    .await
    .unwrap();
    let argv: Vec<String> = serde_json::from_slice(&std::fs::read(&argv_file).unwrap()).unwrap();
    let layers = argv.iter().position(|a| a == "--n-gpu-layers").unwrap();
    assert_eq!(argv[layers + 1], "99");
    let env: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&env_file).unwrap()).unwrap();
    assert!(
        env["LD_LIBRARY_PATH"].as_str().unwrap().starts_with(
            &root
                .path()
                .join("tools/llama-cuda12-fixture")
                .display()
                .to_string()
        ),
        "{env}"
    );
    owned.stop().await;
}
