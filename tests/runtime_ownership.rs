use alt_cli::{
    config::{Preferences, Profile, Provider},
    models, runtime,
};
use std::{
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};
#[path = "fixtures/gguf.rs"]
mod gguf_fixture;
fn cancel() -> models::Cancel {
    Arc::new(AtomicBool::new(false))
}
fn script(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}
async fn setup(root: &Path) -> (Preferences, Profile) {
    let model = root.join("fixture.gguf");
    std::fs::write(&model, gguf_fixture::fixture(None)).unwrap();
    let a = models::import(root, &model, true, cancel(), |_| {})
        .await
        .unwrap();
    let binary = root.join("private-runtime");
    script(
        &binary,
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/private_runtime.py"
        ))
        .unwrap(),
    );
    (
        Preferences {
            runtime_path: Some(binary),
            ..Preferences::default()
        },
        Profile {
            provider: Provider::Openai,
            endpoint: "http://127.0.0.1:1/v1".into(),
            model: "fixture-no-weights".into(),
            context_tokens: 4096,
            max_turns: 2,
            uncensored: true,
            api_key_env: None,
            local_model: Some(a.id),
            inference: None,
        },
    )
}
#[test]
fn explicit_missing_or_nonexecutable_selection_never_uses_discovery() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("tools/llama-b11429")).unwrap();
    script(
        &root.path().join("tools/llama-b11429/llama-server"),
        "#!/bin/sh\nexit 0\n",
    );
    script(&root.path().join("tools/goose"), "#!/bin/sh\nexit 0\n");
    let chosen = root.path().join("chosen");
    let p = Preferences {
        runtime_path: Some(chosen.clone()),
        engine_path: Some(chosen.clone()),
        ..Preferences::default()
    };
    assert!(runtime::find_runtime(root.path(), &p).is_none());
    assert!(runtime::find_engine(root.path(), Path::new("goose"), &p).is_none());
    std::fs::write(&chosen, "not executable").unwrap();
    assert!(runtime::find_runtime(root.path(), &p).is_none());
    assert!(runtime::find_runtime(root.path(), &Preferences::default()).is_some());
}
#[tokio::test]
async fn private_endpoint_one_owner_warm_identity_and_replacement() {
    let root = tempfile::tempdir().unwrap();
    let (p, profile) = setup(root.path()).await;
    let mut r = runtime::LocalRuntime::start(root.path(), &p, &profile, cancel(), |_| {})
        .await
        .unwrap();
    let pid = r.pid().unwrap();
    let endpoint = r.endpoint.clone();
    let client = reqwest::Client::new();
    let response: serde_json::Value = client
        .get(format!("{}/health", endpoint.trim_end_matches("/v1")))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(response["fixture_pid"], pid);
    let u = reqwest::Url::parse(&endpoint).unwrap();
    assert!(
        std::net::TcpListener::bind(("127.0.0.1", u.port().unwrap())).is_err(),
        "Private proxy must retain its listener during ownership"
    );
    assert_eq!(
        client
            .get(format!(
                "http://{}:{}/health",
                u.host_str().unwrap(),
                u.port().unwrap()
            ))
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    let failure = runtime::LocalRuntime::start(root.path(), &p, &profile, cancel(), |_| {})
        .await
        .err()
        .unwrap()
        .to_string();
    assert!(failure.contains("already running"));
    r.check_warm_identity().unwrap();
    let original = std::fs::read(p.runtime_path.as_ref().unwrap()).unwrap();
    std::fs::write(p.runtime_path.as_ref().unwrap(), &original).unwrap();
    assert!(r.check_warm_identity().is_err());
    assert_eq!(r.offload_observation()["offloaded_layers"], 0);
    r.stop().await;
    assert!(!Path::new(&format!("/proc/{pid}")).exists());
    let mut replacement = runtime::LocalRuntime::start(root.path(), &p, &profile, cancel(), |_| {})
        .await
        .unwrap();
    assert_ne!(replacement.endpoint, endpoint);
    replacement.stop().await;
}

#[tokio::test]
async fn configurable_loading_deadline_and_cancellation_reap_owned_child() {
    for cancelled in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let (mut p, profile) = setup(root.path()).await;
        let binary = p.runtime_path.as_ref().unwrap();
        let source = std::fs::read_to_string(binary).unwrap().replace(
            "if os.environ.get('ALT_FIXTURE_NEVER_READY') == '1':",
            "if True:",
        );
        std::fs::write(binary, source).unwrap();
        p.runtime.startup_timeout_secs = 5;
        let signal = cancel();
        if cancelled {
            let signal = signal.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                signal.store(true, std::sync::atomic::Ordering::Relaxed);
            });
        }
        let mut pid = None;
        let started = std::time::Instant::now();
        let error = runtime::LocalRuntime::start_observed(
            root.path(),
            &p,
            &profile,
            signal,
            |_| {},
            |id| pid = Some(id),
        )
        .await
        .err()
        .unwrap()
        .to_string();
        assert!(
            error.contains(if cancelled {
                "cancelled"
            } else {
                "within 5 seconds"
            }),
            "{error}"
        );
        assert!(started.elapsed() < std::time::Duration::from_secs(if cancelled { 3 } else { 8 }));
        assert!(!Path::new(&format!("/proc/{}", pid.unwrap())).exists());
    }
}
#[tokio::test]
async fn process_group_cleanup_retains_helpers_after_leader_exit() {
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("helper.pid");
    let mut command = tokio::process::Command::new("python3");
    command.arg("-c").arg("import subprocess,sys; p=subprocess.Popen(['python3','-c','import signal,time;signal.signal(signal.SIGTERM,signal.SIG_IGN);time.sleep(60)']);open(sys.argv[1],'w').write(str(p.pid))").arg(&marker);
    alt_cli::process::configure(&mut command);
    let mut child = command.spawn().unwrap();
    let mut group = alt_cli::process::OwnedGroup::capture(&child);
    child.wait().await.unwrap();
    let pid: u32 = std::fs::read_to_string(&marker).unwrap().parse().unwrap();
    assert!(alt_cli::jobs::Identity::read(pid).is_some());
    group.stop(&mut child).await;
    for _ in 0..20 {
        if alt_cli::jobs::Identity::read(pid).is_none() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    panic!("Owned helper survived leader-exit cleanup");
}

#[tokio::test]
async fn short_probe_bounds_capture_and_reaps_helpers_on_deadline() {
    let mut command = tokio::process::Command::new("python3");
    command.args(["-c", "print('x'*4096)"]);
    let error =
        alt_cli::process::bounded_output(&mut command, std::time::Duration::from_secs(2), 1024)
            .await
            .unwrap_err()
            .to_string();
    assert!(error.contains("exceeded"), "{error}");
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("helper.pid");
    let mut command = tokio::process::Command::new("python3");
    command.arg("-c").arg("import subprocess,sys; p=subprocess.Popen(['python3','-c','import signal,time;signal.signal(signal.SIGTERM,signal.SIG_IGN);time.sleep(60)']);open(sys.argv[1],'w').write(str(p.pid))").arg(&marker);
    let error =
        alt_cli::process::bounded_output(&mut command, std::time::Duration::from_millis(250), 1024)
            .await
            .unwrap_err()
            .to_string();
    assert!(error.contains("timed out"), "{error}");
    let pid: u32 = std::fs::read_to_string(&marker).unwrap().parse().unwrap();
    for _ in 0..20 {
        if alt_cli::jobs::Identity::read(pid).is_none() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    panic!("Probe helper survived bounded capture");
}
