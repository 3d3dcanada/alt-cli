use alt_cli::{
    config::{Config, Profile, Provider},
    engine::{Engine, Event, response_value},
    store::{Session, Store},
};
use serde_json::{Value, json};
use std::{path::PathBuf, process::Command, time::Duration};

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/acp_engine.py")
}
fn profile() -> Profile {
    Profile {
        provider: Provider::Openai,
        endpoint: "http://127.0.0.1:8080/v1".into(),
        model: "fixture".into(),
        context_tokens: 8192,
        max_turns: 4,
        uncensored: false,
        api_key_env: None,
        local_model: None,
    }
}

async fn connect() -> Engine {
    let mut command = tokio::process::Command::new("python3");
    command.arg(fixture());
    let mut engine = Engine::spawn(command).await.unwrap();
    assert_eq!(engine.initialize().await.unwrap()["protocolVersion"], 1);
    assert_eq!(
        engine
            .call("session/new", json!({"cwd":"/tmp","mcpServers":[]}))
            .await
            .unwrap()["sessionId"],
        "fixture-session"
    );
    engine
}

#[tokio::test]
async fn streaming_permissions_and_large_session_replay() {
    tokio::time::timeout(Duration::from_secs(8), async {
        let mut engine = connect().await;
        let mut response = engine.prompt("fixture-session", "tools").await.unwrap();
        let mut updates = Vec::new();
        let mut permissions = 0;
        loop {
            tokio::select! {
                result = &mut response => { assert_eq!(response_value(result).unwrap()["stopReason"], "end_turn"); break; }
                event = engine.next_event() => match event.unwrap() {
                    Event::Permission {id, params} => { permissions += 1; engine.permission(id, &params, false).await.unwrap(); }
                    Event::Update(value) => updates.push(value),
                    Event::Disconnected(reason) => panic!("{reason}"),
                    _ => {},
                }
            }
        }
        while let Some(event) = engine.try_event() { if let Event::Update(value) = event { updates.push(value); } }
        assert_eq!(permissions, 1);
        let text = updates.iter().filter_map(alt_cli::engine::update_text).collect::<String>();
        assert!(text.contains("denied")); assert!(text.contains("final chunk")); assert!(text.contains("fixture evidence"));
        engine.call("session/load", json!({"sessionId":"fixture-session","cwd":"/tmp","mcpServers":[]})).await.unwrap();
        assert!(engine.try_event().is_none(), "Historical replay leaked into a new turn");
        engine.shutdown().await;
    }).await.expect("ACP test hung");
}

#[tokio::test]
async fn cancellation_and_broken_peers_do_not_hang() {
    tokio::time::timeout(Duration::from_secs(8), async {
        let mut engine = connect().await;
        let response = engine.prompt("fixture-session", "wait").await.unwrap();
        engine.cancel("fixture-session").await.unwrap();
        assert_eq!(
            response_value(response.await).unwrap()["stopReason"],
            "cancelled"
        );
        engine.shutdown().await;
        for prompt in ["crash", "bad-json", "oversize"] {
            let mut engine = connect().await;
            let response = engine.prompt("fixture-session", prompt).await.unwrap();
            assert!(response_value(response.await).is_err(), "{prompt}");
            engine.shutdown().await;
        }
    })
    .await
    .expect("Broken engine hung client");
}

#[test]
fn session_recovery_and_profile_validation() {
    let dir = tempfile::tempdir().unwrap();
    let mut p = profile();
    p.endpoint = "http://secret@example.com/v1".into();
    assert!(p.validate().is_err());
    p = profile();
    p.max_turns = 0;
    assert!(p.validate().is_err());
    Config::create(dir.path(), profile()).unwrap();
    assert!(Config::create(dir.path(), profile()).is_err());
    assert_eq!(
        Config::read(dir.path())
            .unwrap()
            .profile(None)
            .unwrap()
            .1
            .context_tokens,
        8192
    );
    let store = Store::open(dir.path()).unwrap();
    let session = Session {
        id: "s1".into(),
        engine_id: "engine-1".into(),
        cwd: "/tmp".into(),
        profile_name: "local".into(),
        profile: profile(),
    };
    store.create(&session).unwrap();
    store
        .append("s1", &json!({"type":"user","text":"こんにちは"}))
        .unwrap();
    assert!(store.append("missing", &json!({})).is_err());
    drop(store);
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(store.get("s1").unwrap().engine_id, "engine-1");
    assert_eq!(store.history("s1").unwrap()[0]["text"], "こんにちは");
    assert_eq!(store.list().unwrap().len(), 1);
    for i in 0..650 {
        store
            .append("s1", &json!({"type":"fixture", "index":i}))
            .unwrap();
    }
    let recent = store.recent_history("s1").unwrap();
    assert_eq!(recent.len(), 600);
    assert_eq!(recent.first().unwrap()["index"], 50);
    assert_eq!(recent.last().unwrap()["index"], 649);
    assert_eq!(store.history("s1").unwrap().len(), 651);
    store.rename("s1", "A project check").unwrap();
    assert_eq!(store.summaries("project check", false).unwrap().len(), 1);
    store.archive("s1", true).unwrap();
    assert!(store.summaries("", false).unwrap().is_empty());
    assert_eq!(store.summaries("", true).unwrap().len(), 1);
    store.archive("s1", false).unwrap();
    assert_eq!(alt_cli::display_text("\x1b]52;bad\x07ok\n"), "]52;badok\n");
}

#[test]
fn cli_streams_exports_resumes_and_denies_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let cli = || {
        let mut c = Command::new(env!("CARGO_BIN_EXE_alt"));
        c.arg("--data-dir")
            .arg(dir.path())
            .arg("--engine")
            .arg(fixture());
        c
    };
    assert!(
        cli()
            .args(["init", "--model", "fixture"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let first = cli().args(["run", "tools", "--json"]).output().unwrap();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let events: Vec<Value> = String::from_utf8(first.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert!(
        events
            .iter()
            .any(|e| e["type"] == "permission_decision" && e["allow"] == false)
    );
    assert!(
        events
            .iter()
            .any(|e| e["data"]["update"]["content"]["text"] == "final chunk\n")
    );
    assert_eq!(events.last().unwrap()["type"], "turn_end");
    let id = events[0]["session"]["id"].as_str().unwrap();
    let resume = cli()
        .args(["run", "tools", "--resume", id, "--allow-tools"])
        .output()
        .unwrap();
    assert!(
        resume.status.success(),
        "{}",
        String::from_utf8_lossy(&resume.stderr)
    );
    assert!(String::from_utf8_lossy(&resume.stdout).contains("allowed"));
    let export = cli().args(["export", id]).output().unwrap();
    assert!(export.status.success());
    assert!(String::from_utf8_lossy(&export.stdout).contains("permission_decision"));
    let cancelled = cli()
        .args(["run", "wait", "--timeout", "1"])
        .output()
        .unwrap();
    assert!(!cancelled.status.success());
    assert!(String::from_utf8_lossy(&cancelled.stderr).contains("Turn cancelled"));
}
