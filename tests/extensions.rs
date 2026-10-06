use alt_cli::extensions::{self, Connection, Manager, Transport};
use serde_json::json;
fn connection() -> Connection {
    Connection {
        name: "fixture".into(),
        transport: Transport::Stdio {
            command: "python3".into(),
            args: vec![format!(
                "{}/tests/fixtures/mcp_server.py",
                env!("CARGO_MANIFEST_DIR")
            )],
            env_names: vec![],
        },
        enabled: false,
        selected_tools: vec![],
    }
}
#[tokio::test]
async fn selected_stdio_tools_keep_state_and_reconnect_without_replay() {
    let root = tempfile::tempdir().unwrap();
    extensions::save(root.path(), &connection()).unwrap();
    assert_eq!(
        extensions::probe(root.path(), "fixture").await.unwrap()["tools"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(extensions::select(root.path(), "fixture", vec!["invented".into()]).is_err());
    extensions::select(
        root.path(),
        "fixture",
        vec!["counter".into(), "failure".into()],
    )
    .unwrap();
    let manager = Manager::default();
    for expected in ["1", "2"] {
        assert_eq!(
            manager
                .call(root.path(), "fixture", "counter", json!({}))
                .await
                .unwrap()["content"][0]["text"],
            expected
        );
    }
    assert_eq!(
        manager
            .call(root.path(), "fixture", "failure", json!({}))
            .await
            .unwrap()["isError"],
        true
    );
    assert!(
        manager
            .call(
                root.path(),
                "fixture",
                "counter",
                json!({"disconnect":true})
            )
            .await
            .is_err()
    );
    assert_eq!(
        manager
            .call(root.path(), "fixture", "counter", json!({}))
            .await
            .unwrap()["content"][0]["text"],
        "1"
    );
    extensions::select(root.path(), "fixture", vec![]).unwrap();
    assert!(
        manager
            .call(root.path(), "fixture", "counter", json!({}))
            .await
            .is_err()
    );
}
#[tokio::test]
async fn remote_auth_session_and_sse() {
    http_session_fixture(false).await;
}
#[tokio::test]
async fn remote_sse_accepts_multiline_data_and_optional_space() {
    http_session_fixture(true).await;
}
async fn http_session_fixture(variants: bool) {
    use tokio::{
        io::{AsyncBufReadExt, BufReader},
        process::Command,
    };
    let mut command = Command::new("python3");
    command
        .arg(format!(
            "{}/tests/fixtures/mcp_server.py",
            env!("CARGO_MANIFEST_DIR")
        ))
        .arg("--http");
    if variants {
        command.arg("--sse-variants");
    }
    let mut server = command
        .stdout(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(server.stdout.take().unwrap())
        .read_line(&mut line)
        .await
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    let mut c = connection();
    c.transport = Transport::Http {
        url: format!("http://127.0.0.1:{}/mcp", line.trim()),
        auth_env: None,
    };
    extensions::save(root.path(), &c).unwrap();
    assert!(extensions::probe(root.path(), "fixture").await.is_err());
    // Isolated child environment avoids unsafe concurrent set_var in Rust tests.
    let file = root.path().join("extensions/fixture.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    value["transport"]["auth_env"] = json!("ALT_MCP_FIXTURE_TOKEN");
    std::fs::write(file, serde_json::to_vec(&value).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_alt"))
        .args([
            "--data-dir",
            root.path().to_str().unwrap(),
            "extensions",
            "check",
            "fixture",
        ])
        .env("ALT_MCP_FIXTURE_TOKEN", "fixture-secret")
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("counter"));
    server.kill().await.unwrap();
    server.wait().await.unwrap();
}
#[tokio::test]
async fn external_calls_require_actual_alt_approval_and_respect_access() {
    use alt_cli::{
        engine::Event,
        project::{Policy, Project},
        toolbox::Bridge,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let root = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    extensions::save(root.path(), &connection()).unwrap();
    extensions::probe(root.path(), "fixture").await.unwrap();
    extensions::select(root.path(), "fixture", vec!["counter".into()]).unwrap();
    {
        let p = Project::open(root.path(), cwd.path()).unwrap();
        p.start_task("t", "test external approval").unwrap();
    }
    for (policy, allow) in [
        (Policy::Guided, false),
        (Policy::Trusted, false),
        (Policy::Trusted, true),
    ] {
        let mut bridge = Bridge::start(root.path(), cwd.path(), policy).unwrap();
        bridge.context("t", "s");
        let path = bridge.path.clone();
        let call = tokio::spawn(async move {
            let mut s = tokio::net::UnixStream::connect(path).await.unwrap();
            let mut b=json!({"name":"extension","arguments":{"connection":"fixture","tool":"counter","arguments":{}}}).to_string();
            b.push('\n');
            s.write_all(b.as_bytes()).await.unwrap();
            let mut out = Vec::new();
            s.read_to_end(&mut out).await.unwrap();
            serde_json::from_slice::<serde_json::Value>(&out).unwrap()
        });
        if policy == Policy::Trusted {
            loop {
                let e =
                    tokio::time::timeout(std::time::Duration::from_secs(5), bridge.events.recv())
                        .await
                        .unwrap()
                        .unwrap();
                if let Event::Permission { id, .. } = e {
                    assert!(bridge.decision(id.as_str().unwrap(), allow));
                    break;
                }
            }
        }
        let out = tokio::time::timeout(std::time::Duration::from_secs(5), call)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(out["isError"].as_bool().unwrap_or(false), !allow, "{out}");
    }
}
