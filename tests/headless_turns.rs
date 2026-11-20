//! Headless turns against a slow provider: waiting lines while a request is in
//! flight, and a timeout that ends cleanly even when the engine never stops.
#[path = "fixtures/upstream.rs"]
mod upstream;

use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{Duration, Instant},
};
use upstream::Reply;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/acp_engine.py")
}

fn cli(data: &Path, project: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_alt"));
    c.arg("--data-dir")
        .arg(data)
        .arg("--engine")
        .arg(fixture())
        .current_dir(project);
    c
}

fn setup(endpoint: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let init = cli(dir.path(), &project)
        .args(["init", "--model", "fixture", "--endpoint", endpoint])
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    (dir, project)
}

fn run(data: &Path, project: &Path, args: &[&str]) -> Output {
    cli(data, project).arg("run").args(args).output().unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_slow_provider_gets_waiting_lines_in_text_and_json() {
    let answer = json!({"choices":[{"message":{"role":"assistant","content":"hi"},"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":1}});
    // Two upstreams: the fixture serves one connection at a time.
    let slow_json = upstream::start(vec![
        Reply::json(answer.clone()).after(Duration::from_secs(17)),
    ])
    .await;
    let slow_text = upstream::start(vec![Reply::json(answer).after(Duration::from_secs(17))]).await;
    let (json_dir, json_project) = setup(&slow_json.endpoint);
    let (text_dir, text_project) = setup(&slow_text.endpoint);
    let (json_run, text_run) = tokio::join!(
        tokio::task::spawn_blocking(move || {
            let out = run(json_dir.path(), &json_project, &["slow-model", "--json"]);
            (out, json_dir)
        }),
        tokio::task::spawn_blocking(move || {
            let out = run(text_dir.path(), &text_project, &["slow-model"]);
            (out, text_dir)
        }),
    );
    let (json_out, _json_dir) = json_run.unwrap();
    let (text_out, _text_dir) = text_run.unwrap();
    assert!(
        json_out.status.success(),
        "{}",
        String::from_utf8_lossy(&json_out.stderr)
    );
    let events: Vec<Value> = String::from_utf8_lossy(&json_out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let waiting: Vec<&Value> = events.iter().filter(|e| e["type"] == "waiting").collect();
    assert!(!waiting.is_empty(), "no waiting event in {events:?}");
    assert_eq!(waiting[0]["model"], "fixture");
    assert_eq!(waiting[0]["stage"], "waiting_for_output");
    assert!(waiting[0]["elapsed_s"].as_u64().unwrap() >= 15);
    assert_eq!(waiting[0]["retries"], 0);
    assert_eq!(events.last().unwrap()["type"], "turn_end");
    assert!(
        text_out.status.success(),
        "{}",
        String::from_utf8_lossy(&text_out.stderr)
    );
    let stderr = String::from_utf8_lossy(&text_out.stderr);
    assert!(stderr.contains("Waiting for fixture: 15 s"), "{stderr}");
    assert!(
        !String::from_utf8_lossy(&text_out.stdout).contains("Waiting for"),
        "waiting lines stay off stdout in text mode"
    );
}

#[test]
fn a_timeout_ends_cleanly_even_when_the_engine_never_stops() {
    let (dir, project) = setup("http://127.0.0.1:9/v1");
    let started = Instant::now();
    let out = run(dir.path(), &project, &["stuck", "--timeout", "1", "--json"]);
    let elapsed = started.elapsed();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(124), "{stderr}");
    assert!(
        stderr.contains("Turn stopped by --timeout 1 s; no files were changed"),
        "{stderr}"
    );
    assert!(!stderr.contains("did not stop"), "{stderr}");
    assert!(elapsed < Duration::from_secs(30), "{elapsed:?}");
    let timeout = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .find(|e| e["type"] == "timeout")
        .expect("typed timeout event");
    assert_eq!(timeout["timeout_s"], 1);
    assert_eq!(timeout["changes"], json!([]));
}

#[test]
fn a_session_without_alt_tools_is_refused_before_any_prompt() {
    let (dir, project) = setup("http://127.0.0.1:9/v1");
    let trace = dir.path().join("trace.jsonl");
    let out = cli(dir.path(), &project)
        .env("ALT_FIXTURE_NO_TOOLS", "1")
        .env("ALT_FIXTURE_TRACE", &trace)
        .args(["run", "hello"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(
        stderr.contains("tool server did not attach") && stderr.contains("No model request"),
        "{stderr}"
    );
    let methods = std::fs::read_to_string(&trace).unwrap();
    assert!(methods.contains("session/new"), "{methods}");
    assert!(!methods.contains("session/prompt"), "{methods}");
    // The same fixture with the handshake runs the turn.
    let ok = run(dir.path(), &project, &["hello"]);
    assert!(
        ok.status.success(),
        "{}",
        String::from_utf8_lossy(&ok.stderr)
    );
}

#[test]
fn doctor_prints_every_line_without_an_engine_and_never_runs_one_from_path() {
    let (dir, project) = setup("http://127.0.0.1:9/v1");
    let bin = dir.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    let marker = dir.path().join("stray-goose-ran");
    let stray = bin.join("goose");
    std::fs::write(&stray, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&stray, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    let out = Command::new(env!("CARGO_BIN_EXE_alt"))
        .arg("--data-dir")
        .arg(dir.path())
        .arg("doctor")
        .env("PATH", path)
        .current_dir(&project)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stdout}{stderr}");
    for expected in [
        "Endpoint UNREACHABLE",
        "Engine: NOT FOUND",
        "alt install engine",
        "Isolated checks (Guided): ",
        "Access mode: Guided changes",
        "Checks configured in this project: none",
        "Full access: the terminal and checks run on this computer",
    ] {
        assert!(stdout.contains(expected), "{expected} missing:\n{stdout}");
    }
    assert!(!stdout.contains("unknown unknown"), "{stdout}");
    assert!(stderr.contains("alt doctor found a problem"), "{stderr}");
    assert!(!marker.exists(), "doctor ran the goose found on PATH");
}

#[test]
fn doctor_labels_an_engine_that_is_not_the_tested_version() {
    let (dir, project) = setup("http://127.0.0.1:9/v1");
    let out = cli(dir.path(), &project).arg("doctor").output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!out.status.success(), "{stdout}");
    assert!(
        stdout.contains("fixture 1 · ACP v1 · Alt tools attached"),
        "{stdout}"
    );
    assert!(
        stdout.contains("NOT the tested Goose 1.53.0. Run `alt install engine`"),
        "{stdout}"
    );
}
