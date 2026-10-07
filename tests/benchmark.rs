use alt_cli::{
    benchmark,
    config::{Preferences, Profile, Provider},
    models, runtime,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
fn profile(endpoint: String) -> Profile {
    Profile {
        provider: Provider::Openai,
        endpoint,
        model: "protocol-fixture-no-weights".into(),
        context_tokens: 4096,
        max_turns: 2,
        uncensored: true,
        api_key_env: None,
        local_model: None,
        inference: None,
    }
}
#[tokio::test]
async fn cancellation_interrupts_a_stalled_backend_and_preserves_a_failed_report() {
    let root = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let profile = profile(format!("http://{}/v1", listener.local_addr().unwrap()));
    let cancel = Arc::new(AtomicBool::new(false));
    let signal = cancel.clone();
    let server = tokio::spawn(async move {
        let (_socket, _) = listener.accept().await.unwrap();
        signal.store(true, Ordering::Relaxed);
        std::future::pending::<()>().await;
    });
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        benchmark::run(root.path(), &Preferences::default(), &profile, cancel),
    )
    .await
    .unwrap()
    .unwrap();
    server.abort();
    assert_eq!(result["error"], "Benchmark cancelled");
    assert!(result["sampled_peak_runtime_rss_bytes"].is_null());
    assert_eq!(result["model"], "protocol-fixture-no-weights");
    assert_eq!(
        std::fs::read_dir(root.path().join("evaluations"))
            .unwrap()
            .count(),
        1
    );
}
#[tokio::test]
async fn requested_gpu_and_allocation_failure_explain_recovery_without_substituting_models() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("fixture.gguf");
    std::fs::write(&original, b"GGUFfixture-not-real-weights").unwrap();
    let model = models::import(
        root.path(),
        &original,
        true,
        Arc::new(AtomicBool::new(false)),
        |_| {},
    )
    .await
    .unwrap();
    let executable = root.path().join("fake-runtime");
    std::fs::write(&executable, "#!/bin/sh\nif [ \"$1\" = --list-devices ]; then echo 'Available devices:'; exit 0; fi\necho 'failed to allocate model memory' >&2\nexit 1\n").unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut preferences = Preferences {
        runtime_path: Some(executable),
        ..Preferences::default()
    };
    let mut selected = profile("http://127.0.0.1:1/v1".into());
    selected.local_model = Some(model.id.clone());
    preferences.runtime.gpu_layers = 1;
    let result = runtime::LocalRuntime::start(
        root.path(),
        &preferences,
        &selected,
        Arc::new(AtomicBool::new(false)),
        |_| {},
    )
    .await;
    assert!(
        result
            .err()
            .unwrap()
            .to_string()
            .contains("did not report a usable accelerator")
    );
    preferences.runtime.gpu_layers = 0;
    let runtime_path = preferences.runtime_path.as_ref().unwrap();
    let allocation_fixture = std::fs::read(runtime_path).unwrap();
    std::fs::write(runtime_path, "#!/bin/sh\necho 'error while loading shared libraries: libgomp.so.1: cannot open shared object file' >&2\nexit 127\n").unwrap();
    let missing_library = runtime::LocalRuntime::start(
        root.path(),
        &preferences,
        &selected,
        Arc::new(AtomicBool::new(false)),
        |_| {},
    )
    .await;
    let message = missing_library.err().unwrap().to_string();
    assert!(message.contains("libgomp1") && message.contains("selected model was not changed"));
    std::fs::write(runtime_path, allocation_fixture).unwrap();
    let result = benchmark::run(
        root.path(),
        &preferences,
        &selected,
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .unwrap();
    assert!(
        result["error"]
            .as_str()
            .unwrap()
            .contains("allocate model memory"),
        "{result}"
    );
    assert_eq!(result["model"], selected.model);
    assert_eq!(result["artifact"]["id"], model.id);
    assert!(
        result["recovery"]
            .as_str()
            .unwrap()
            .contains("no automatic substitution")
    );
}

#[tokio::test]
async fn qualification_cli_returns_without_opening_an_agent_and_keeps_external_scope_explicit() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let root = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let selected = profile(format!("http://{}/v1", listener.local_addr().unwrap()));
    alt_cli::config::Config::create(root.path(), selected).unwrap();
    let server = tokio::spawn(async move {
        for _ in 0..6 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut data = Vec::new();
            let mut part = [0; 1024];
            loop {
                let n = socket.read(&mut part).await.unwrap();
                assert!(n > 0);
                data.extend_from_slice(&part[..n]);
                if let Some(end) = data.windows(4).position(|x| x == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&data[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_lowercase()
                                .strip_prefix("content-length:")
                                .map(|s| s.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    if data.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let response=serde_json::json!({"choices":[{"message":{"content":"Protocol fixture; no weights"}}],"usage":{"completion_tokens":16}}).to_string();
            socket.write_all(format!("HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",response.len(),response).as_bytes()).await.unwrap();
        }
    });
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        tokio::process::Command::new(env!("CARGO_BIN_EXE_alt"))
            .arg("--data-dir")
            .arg(root.path())
            .args([
                "--engine",
                "/missing-engine",
                "qualify",
                "--contexts",
                "2048,4096",
            ])
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["status"], "completed");
    assert_eq!(report["rows"].as_array().unwrap().len(), 2);
    for row in report["rows"].as_array().unwrap() {
        assert_eq!(row["lifecycle"]["status"], "unmeasured");
        assert!(row["measurement"]["observed_native_context"].is_null());
        assert_eq!(
            row["measurement"]["configuration_identity"]["identity_complete"],
            false
        );
    }
    server.await.unwrap();
    let empty = tempfile::tempdir().unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_alt"))
        .arg("--data-dir")
        .arg(empty.path())
        .args(["tools", "coding"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        Preferences::load(empty.path()).unwrap().tool_profile,
        alt_cli::toolbox::ToolProfile::Coding
    );
    for (arguments, expected) in [
        (vec!["runtime", "--thinking", "false"], Some(false)),
        (vec!["runtime", "--thinking", "true"], Some(true)),
        (vec!["runtime", "--thinking-default"], None),
    ] {
        let result = std::process::Command::new(env!("CARGO_BIN_EXE_alt"))
            .arg("--data-dir")
            .arg(empty.path())
            .args(arguments)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            Preferences::load(empty.path()).unwrap().runtime.thinking,
            expected
        );
    }
}
