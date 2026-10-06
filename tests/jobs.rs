use alt_cli::jobs;
use serde_json::json;
use std::{path::Path, process::Command, time::Duration};
fn start(root: &Path, command: &str, timeout: u64) -> jobs::Record {
    let r = Command::new(env!("CARGO_BIN_EXE_alt"))
        .args(["--data-dir"])
        .arg(root)
        .args([
            "--access",
            "trusted",
            "jobs",
            "start",
            command,
            "--timeout",
            &timeout.to_string(),
        ])
        .output()
        .unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    serde_json::from_slice(&r.stdout).unwrap()
}
struct Cleanup<'a>(&'a Path, String);
impl Drop for Cleanup<'_> {
    fn drop(&mut self) {
        let _ = Command::new(env!("CARGO_BIN_EXE_alt"))
            .arg("--data-dir")
            .arg(self.0)
            .args(["jobs", "stop", &self.1])
            .output();
    }
}
#[tokio::test]
async fn persistent_pty_accepts_input_resize_and_reconnect_then_stops() {
    let root = tempfile::tempdir().unwrap();
    let r = start(
        root.path(),
        "printf 'READY\\n'; read value; printf 'VALUE:%s\\n' \"$value\"; stty -icanon -echo; sleep 30",
        0,
    );
    let _cleanup = Cleanup(root.path(), r.id.clone());
    assert!(r.spec.keep);
    assert_eq!(jobs::record(root.path(), &r.id).unwrap().status, "running");
    jobs::request(
        root.path(),
        &r.id,
        json!({"action":"resize","rows":30,"cols":90}),
    )
    .await
    .unwrap();
    jobs::request(
        root.path(),
        &r.id,
        json!({"action":"input","text":"hello-terminal\n"}),
    )
    .await
    .unwrap();
    let mut found = false;
    for _ in 0..50 {
        let v = jobs::request(root.path(), &r.id, json!({"action":"snapshot"}))
            .await
            .unwrap();
        if v["screen"]
            .as_str()
            .unwrap()
            .contains("VALUE:hello-terminal")
        {
            found = true;
            assert_eq!(v["record"]["spec"]["rows"], 30);
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(found);
    let mut full = false;
    for _ in 0..64 {
        let result = jobs::request(
            root.path(),
            &r.id,
            json!({"action":"input","text":"x".repeat(8192)}),
        )
        .await;
        if let Err(error) = result {
            assert!(error.to_string().contains("input is full"), "{error}");
            full = true;
            break;
        }
    }
    assert!(full, "Unread PTY input should hit the bounded queue");
    let result = tokio::time::timeout(Duration::from_secs(3), jobs::stop(root.path(), &r.id))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.status, "stopped");
    assert!(!result.process.unwrap().alive());
    assert!(
        jobs::output(root.path(), &r.id)
            .unwrap()
            .contains("VALUE:hello-terminal")
    );
}
#[tokio::test]
async fn timeout_and_exit_are_saved_and_pid_reuse_cannot_signal_another_process() {
    let root = tempfile::tempdir().unwrap();
    let r = start(root.path(), "echo before-timeout; sleep 30", 1);
    let _cleanup = Cleanup(root.path(), r.id.clone());
    for _ in 0..100 {
        if jobs::record(root.path(), &r.id).unwrap().status != "running" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    let end = jobs::record(root.path(), &r.id).unwrap();
    assert_eq!(end.status, "timed out");
    assert!(!end.process.as_ref().unwrap().alive());
    let mut fake = end.clone();
    fake.id = uuid::Uuid::new_v4().to_string();
    fake.status = "running".into();
    fake.supervisor = jobs::Identity::read(std::process::id());
    fake.supervisor.as_mut().unwrap().start = "not-this-process".into();
    fake.process = jobs::Identity::read(std::process::id());
    let directory = root.path().join("jobs").join(&fake.id);
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(
        directory.join("job.json"),
        serde_json::to_vec(&fake).unwrap(),
    )
    .unwrap();
    assert_eq!(
        jobs::stop(root.path(), &fake.id).await.unwrap().status,
        "interrupted"
    );
    assert!(jobs::Identity::read(std::process::id()).unwrap().alive());
}
#[test]
fn control_keys_are_bytes_not_numeric_strings() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    assert_eq!(
        jobs::key_text(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)).unwrap(),
        "\x03"
    );
}

#[tokio::test]
async fn disconnected_control_client_does_not_terminate_persistent_job() {
    use std::{io::Write, net::Shutdown, os::unix::net::UnixStream};
    let root = tempfile::tempdir().unwrap();
    let r = start(root.path(), "sleep 30", 0);
    let _cleanup = Cleanup(root.path(), r.id.clone());
    let socket = root.path().join("jobs").join(&r.id).join("control.sock");
    let mut abandoned = UnixStream::connect(socket).unwrap();
    // Model a closed TUI/SSH connection after sending a request, before reading.
    abandoned.shutdown(Shutdown::Read).unwrap();
    abandoned.write_all(b"{\"action\":\"snapshot\"}\n").unwrap();
    abandoned.shutdown(Shutdown::Write).unwrap();
    let reply = jobs::request(root.path(), &r.id, json!({"action":"snapshot"}))
        .await
        .expect("Another client must still be able to attach");
    assert_eq!(reply["record"]["status"], "running");
    assert!(r.process.unwrap().alive());
    assert_eq!(
        jobs::stop(root.path(), &r.id).await.unwrap().status,
        "stopped"
    );
}

#[tokio::test]
async fn exiting_owner_stops_only_owned_jobs_and_large_output_is_bounded() {
    let root = tempfile::tempdir().unwrap();
    let persistent = start(root.path(), "sleep 40", 0);
    let _cleanup = Cleanup(root.path(), persistent.id.clone());
    let mut owner = Command::new(env!("CARGO_BIN_EXE_alt"))
        .arg("--data-dir")
        .arg(root.path())
        .args(["--access", "trusted", "jobs", "start", "sleep 40", "--wait"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut owned = None;
    for _ in 0..100 {
        owned = jobs::list(root.path())
            .unwrap()
            .into_iter()
            .find(|r| !r.spec.keep && r.status == "running");
        if owned.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let owned = owned.expect("owned job started");
    let _owned_cleanup = Cleanup(root.path(), owned.id.clone());
    owner.kill().unwrap();
    owner.wait().unwrap();
    for _ in 0..100 {
        if jobs::record(root.path(), &owned.id).unwrap().status != "running" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let stopped = jobs::record(root.path(), &owned.id).unwrap();
    assert_eq!(stopped.status, "owner exited");
    assert!(!stopped.process.unwrap().alive());
    assert_eq!(
        jobs::record(root.path(), &persistent.id).unwrap().status,
        "running"
    );
    let noisy = start(
        root.path(),
        "python3 -c 'import sys; sys.stdout.write(\"x\" * (9 * 1024 * 1024))'",
        20,
    );
    let _noisy_cleanup = Cleanup(root.path(), noisy.id.clone());
    for _ in 0..1000 {
        if jobs::record(root.path(), &noisy.id).unwrap().status != "running" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let end = jobs::record(root.path(), &noisy.id).unwrap();
    assert_eq!(end.status, "completed");
    assert!(end.output_truncated);
    assert_eq!(
        std::fs::metadata(root.path().join("jobs").join(&noisy.id).join("output.log"))
            .unwrap()
            .len(),
        8 * 1024 * 1024
    );
}
