use alt_cli::project::{CheckSpec, Policy, Project, digest};
use serde_json::json;
use std::{
    fs,
    sync::{Arc, atomic::AtomicBool},
};
fn begin(p: &Project) {
    p.start_task("task", "Repair without changing assertions")
        .unwrap();
    p.note(
        "task",
        "plan",
        "Read, reproduce, edit implementation, rerun unchanged tests",
        "user",
    )
    .unwrap();
}
#[test]
fn enforced_reads_paths_checkpoints_undo_and_conflicts() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    fs::write(cwd.path().join("calc.py"), "return a-b\n").unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    begin(&p);
    assert!(
        p.prepare_edit(
            "task",
            "calc.py",
            Some(&digest(b"return a-b\n")),
            "a-b",
            "a+b",
            "replace",
            "Fix sum"
        )
        .is_err()
    );
    let read = p.read("task", "calc.py", 1, 100).unwrap();
    let hash = read["sha256"].as_str().unwrap();
    assert!(p.read("task", "../outside", 1, 100).is_err());
    assert!(p.read("task", "/etc/passwd", 1, 100).is_err());
    assert!(
        p.prepare_edit(
            "task",
            "calc.py",
            Some(hash),
            "",
            "replacement",
            "create",
            "Replace whole file"
        )
        .is_err()
    );
    let c = p
        .prepare_edit(
            "task",
            "calc.py",
            Some(hash),
            "a-b",
            "a+b",
            "replace",
            "Fix sum",
        )
        .unwrap();
    assert!(c.diff().contains("+return a+b"));
    assert!(p.apply(&c.id, Policy::ReviewOnly).is_err());
    p.apply(&c.id, Policy::Guided).unwrap();
    assert_eq!(
        fs::read_to_string(cwd.path().join("calc.py")).unwrap(),
        "return a+b\n"
    );
    assert!(
        p.apply(&c.id, Policy::Guided).is_err(),
        "Do not repeat a mutation after lost responses"
    );
    fs::write(cwd.path().join("calc.py"), "manual change\n").unwrap();
    assert!(p.undo(&c.id).is_err());
    assert_eq!(
        fs::read_to_string(cwd.path().join("calc.py")).unwrap(),
        "manual change\n"
    );
    fs::write(cwd.path().join("calc.py"), "return a+b\n").unwrap();
    p.undo(&c.id).unwrap();
    assert_eq!(
        fs::read_to_string(cwd.path().join("calc.py")).unwrap(),
        "return a-b\n"
    );
    drop(p);
    let p = Project::open(data.path(), cwd.path()).unwrap();
    assert_eq!(p.change(&c.id).unwrap().status, "undone");
}
#[test]
fn stale_edits_symlinks_creation_deletion_and_atomic_task_undo() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    begin(&p);
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/tmp", cwd.path().join("escape")).unwrap();
        assert!(p.read("task", "escape/file", 1, 20).is_err());
        assert!(
            p.prepare_edit("task", "escape/file", None, "", "x", "create", "Escape")
                .is_err()
        );
    }
    let a = p
        .prepare_edit("task", "new.py", None, "", "original\n", "create", "Create")
        .unwrap();
    p.apply(&a.id, Policy::Guided).unwrap();
    let hash = p.read("task", "new.py", 1, 100).unwrap()["sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    let b = p
        .prepare_edit(
            "task",
            "new.py",
            Some(&hash),
            "original",
            "changed",
            "replace",
            "Change",
        )
        .unwrap();
    fs::write(cwd.path().join("new.py"), "manual").unwrap();
    assert!(p.apply(&b.id, Policy::Guided).is_err());
    fs::write(cwd.path().join("new.py"), "original\n").unwrap();
    p.apply(&b.id, Policy::Guided).unwrap();
    let c = p
        .prepare_edit(
            "task",
            "other.py",
            None,
            "",
            "untouched\n",
            "create",
            "Create other",
        )
        .unwrap();
    p.apply(&c.id, Policy::Guided).unwrap();
    fs::write(cwd.path().join("new.py"), "manual").unwrap();
    assert!(p.undo_task("task").is_err());
    assert!(
        cwd.path().join("other.py").exists(),
        "Preflight prevents partial undo"
    );
    fs::write(cwd.path().join("new.py"), "changed\n").unwrap();
    p.undo_task("task").unwrap();
    assert!(!cwd.path().join("new.py").exists());
    assert!(!cwd.path().join("other.py").exists());
    fs::write(cwd.path().join("keep.txt"), "uncommitted user data").unwrap();
    let hash = p.read("task", "keep.txt", 1, 100).unwrap()["sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    let d = p
        .prepare_edit(
            "task",
            "keep.txt",
            Some(&hash),
            "",
            "",
            "delete",
            "Delete by request",
        )
        .unwrap();
    p.apply(&d.id, Policy::Guided).unwrap();
    assert!(!cwd.path().join("keep.txt").exists());
    p.undo(&d.id).unwrap();
    assert_eq!(
        fs::read_to_string(cwd.path().join("keep.txt")).unwrap(),
        "uncommitted user data"
    );
}
#[test]
fn memory_invalidation_failure_budget_and_crash_recovery() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    fs::write(cwd.path().join("a.rs"), "fn previous_symbol() {}\n").unwrap();
    let mut p = Project::open(data.path(), cwd.path()).unwrap();
    begin(&p);
    assert!(!p.search("previous_symbol").unwrap().is_empty());
    fs::write(cwd.path().join("a.rs"), "fn current_symbol() {}\n").unwrap();
    assert!(p.search("previous_symbol").unwrap().is_empty());
    assert!(!p.search("current_symbol").unwrap().is_empty());
    for _ in 0..3 {
        p.attempt("task", "same", Some(true)).unwrap();
    }
    assert!(p.attempt("task", "same", None).is_err());
    assert!(
        p.memory("task", "current_symbol", 1000)
            .unwrap()
            .chars()
            .count()
            <= 1000
    );
    let h = p.read("task", "a.rs", 1, 10).unwrap()["sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    let c = p
        .prepare_edit(
            "task",
            "a.rs",
            Some(&h),
            "current_symbol",
            "new_symbol",
            "replace",
            "Rename",
        )
        .unwrap();
    let database = p.state.join("project.db");
    drop(p);
    let db = rusqlite::Connection::open(database).unwrap();
    let mut payload = serde_json::to_value(&c).unwrap();
    payload["status"] = json!("applying");
    db.execute(
        "UPDATE changes SET status='applying',payload=?1",
        [payload.to_string()],
    )
    .unwrap();
    drop(db);
    fs::write(cwd.path().join("a.rs"), c.after.unwrap()).unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    assert_eq!(p.change(&c.id).unwrap().status, "applied");
}
#[tokio::test]
async fn deterministic_checks_bind_evidence_to_source_and_preserve_original() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    fs::write(cwd.path().join("check.py"),"from pathlib import Path\nPath('generated.txt').write_text('copy only')\nprint('CHECK_OK')\n").unwrap();
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        begin(&p);
        p.set_check(&CheckSpec {
            name: "test".into(),
            argv: vec!["python3".into(), "check.py".into()],
            timeout_secs: 5,
            contract: alt_cli::verification::Contract {
                inputs: alt_cli::verification::ExecutionInputs {
                    generated: vec!["generated.txt".into()],
                    ..Default::default()
                },
                ..Default::default()
            },
        })
        .unwrap();
    }
    let c = alt_cli::sandbox::run(
        data.path().into(),
        cwd.path().into(),
        "task".into(),
        "test".into(),
        Policy::Trusted,
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .unwrap();
    assert_eq!(c.exit_code, Some(0));
    assert!(c.output.contains("CHECK_OK"));
    assert!(!cwd.path().join("generated.txt").exists());
    {
        let mut p = Project::open(data.path(), cwd.path()).unwrap();
        assert!(
            p.task("task")
                .unwrap()
                .check_status
                .contains("passed on current")
        );
        for _ in 0..20 {
            p.note(
                "task",
                "observation",
                &"Old unverified detail. ".repeat(80),
                "model note (unverified)",
            )
            .unwrap();
        }
        p.note("task", "next", "step-old", "model note (unverified)")
            .unwrap();
        p.note("task", "next", "step-new", "model note (unverified)")
            .unwrap();
        let memory = p.memory("task", "check", 4096).unwrap();
        assert!(memory.contains("passed on current files"));
        assert!(
            memory.contains(&c.id),
            "Recent evidence must survive a crowded history"
        );
        assert!(memory.contains("CHECK_OK"));
        let history = memory.split("HISTORICAL events").nth(1).unwrap();
        assert!(
            history.find("step-old").unwrap() < history.find("step-new").unwrap(),
            "History must preserve chronological order"
        );
        assert!(memory.chars().count() <= 4096);
    }
    fs::write(cwd.path().join("other.txt"), "later change").unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    assert!(p.task("task").unwrap().check_status.contains("stale"));
}
#[tokio::test]
async fn no_silent_isolation_fallback_and_terminal_is_explicit() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        begin(&p);
        p.set_check(&CheckSpec {
            name: "test".into(),
            argv: vec!["/bin/true".into()],
            timeout_secs: 5,
            contract: Default::default(),
        })
        .unwrap();
    }
    let available = alt_cli::sandbox::probe().await.available;
    let r = alt_cli::sandbox::run(
        data.path().into(),
        cwd.path().into(),
        "task".into(),
        "test".into(),
        Policy::Guided,
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .unwrap();
    if available {
        assert_eq!(r.exit_code, Some(0));
    } else {
        assert!(r.error.unwrap().contains("No command ran"));
        assert_eq!(r.exit_code, None);
    }
    assert!(
        alt_cli::sandbox::run(
            data.path().into(),
            cwd.path().into(),
            "task".into(),
            "test".into(),
            Policy::ReviewOnly,
            Arc::new(AtomicBool::new(false))
        )
        .await
        .is_err()
    );
}

async fn bridge_call(
    socket: std::path::PathBuf,
    name: &str,
    arguments: serde_json::Value,
) -> serde_json::Value {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    let mut stream = tokio::net::UnixStream::connect(socket).await.unwrap();
    stream
        .write_all(format!("{}\n", json!({"name":name,"arguments":arguments})).as_bytes())
        .await
        .unwrap();
    let mut line = String::new();
    tokio::io::BufReader::new(stream)
        .read_line(&mut line)
        .await
        .unwrap();
    serde_json::from_str(&line).unwrap()
}
#[tokio::test]
async fn native_tool_bridge_cannot_bypass_approval_or_stale_checks() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    fs::write(cwd.path().join("app.py"), "return sum(items) + 1\n").unwrap();
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        begin(&p);
    }
    let mut bridge =
        alt_cli::toolbox::Bridge::start(data.path(), cwd.path(), Policy::Guided).unwrap();
    bridge.context("task", "session");
    let r = bridge_call(bridge.path.clone(), "read", json!({"path":"app.py"})).await;
    assert_eq!(r["isError"], false);
    let args = json!({"path":"app.py","operation":"replace","old_text":"sum(items) + 1","new_text":"sum(items)","reason":"Fix the implementation"});
    let socket = bridge.path.clone();
    let input = args.clone();
    let mut call = tokio::spawn(async move { bridge_call(socket, "edit", input).await });
    let event = tokio::select! {v=bridge.events.recv()=>v.unwrap(),v=&mut call=>panic!("Tool ended before requesting permission: {v:?}"),_=tokio::time::sleep(std::time::Duration::from_secs(5))=>panic!("Permission request timed out")};
    let alt_cli::engine::Event::Permission { id, params } = event else {
        panic!("Missing approval")
    };
    assert_eq!(params["sessionId"], "session");
    assert!(
        params["toolCall"]["content"]
            .to_string()
            .contains("Checkpoint")
    );
    assert_eq!(
        fs::read_to_string(cwd.path().join("app.py")).unwrap(),
        "return sum(items) + 1\n"
    );
    bridge.decision(id.as_str().unwrap(), false);
    assert_eq!(call.await.unwrap()["isError"], true);
    // Changing the source while an approval is visible invalidates the prepared edit.
    let socket = bridge.path.clone();
    let input = args.clone();
    let mut call = tokio::spawn(async move { bridge_call(socket, "edit", input).await });
    let alt_cli::engine::Event::Permission { id, .. } = (tokio::select! {v=bridge.events.recv()=>v.unwrap(),v=&mut call=>panic!("Tool ended before second permission: {v:?}"),_=tokio::time::sleep(std::time::Duration::from_secs(5))=>panic!("Second permission timed out")})
    else {
        panic!("Missing approval")
    };
    fs::write(cwd.path().join("app.py"), "manual edit").unwrap();
    bridge.decision(id.as_str().unwrap(), true);
    assert_eq!(call.await.unwrap()["isError"], true);
    assert_eq!(
        fs::read_to_string(cwd.path().join("app.py")).unwrap(),
        "manual edit"
    );
    // Permission does not change the selected execution profile.
    let denied = bridge_call(
        bridge.path.clone(),
        "terminal",
        json!({"command":"touch should-not-exist"}),
    )
    .await;
    assert_eq!(denied["isError"], true);
    assert!(!cwd.path().join("should-not-exist").exists());
    assert_eq!(
        bridge_call(
            bridge.path.clone(),
            "read",
            json!({"path":"app.py","unexpected":true})
        )
        .await["isError"],
        true
    );
}
#[tokio::test]
async fn check_timeout_cancellation_and_early_exit_kill_background_children() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let pidfile = data.path().join("owned.pid");
    let code = format!(
        "import subprocess\nfrom pathlib import Path\np=subprocess.Popen(['sleep','30'])\nPath({:?}).write_text(str(p.pid))\n",
        pidfile.to_string_lossy()
    );
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        begin(&p);
        p.set_check(&CheckSpec {
            name: "spawn".into(),
            argv: vec!["python3".into(), "-c".into(), code],
            timeout_secs: 2,
            contract: Default::default(),
        })
        .unwrap();
        p.set_check(&CheckSpec {
            name: "timeout".into(),
            argv: vec!["sleep".into(), "30".into()],
            timeout_secs: 1,
            contract: Default::default(),
        })
        .unwrap();
    }
    let result = alt_cli::sandbox::run(
        data.path().into(),
        cwd.path().into(),
        "task".into(),
        "spawn".into(),
        Policy::Trusted,
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .unwrap();
    assert_eq!(result.exit_code, Some(0));
    let pid = fs::read_to_string(pidfile).unwrap();
    let status = fs::read_to_string(format!("/proc/{pid}/status")).unwrap_or_default();
    assert!(
        status.is_empty() || status.contains("State:\tZ"),
        "Background process survived leader exit"
    );
    let timeout = alt_cli::sandbox::run(
        data.path().into(),
        cwd.path().into(),
        "task".into(),
        "timeout".into(),
        Policy::Trusted,
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .unwrap();
    assert!(timeout.timed_out);
    assert_ne!(timeout.exit_code, Some(0));
    let cancelled = alt_cli::sandbox::run(
        data.path().into(),
        cwd.path().into(),
        "task".into(),
        "timeout".into(),
        Policy::Trusted,
        Arc::new(AtomicBool::new(true)),
    )
    .await
    .unwrap();
    assert!(cancelled.cancelled);
    assert_ne!(cancelled.exit_code, Some(0));
}
