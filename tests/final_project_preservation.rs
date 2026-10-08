use alt_cli::project::{Policy, Project};
use std::{fs, path::Path, process::Command};

fn prepare(p: &Project, task: &str, old: &str, new: &str) -> String {
    p.start_task(task, "Preserve existing work").unwrap();
    p.note(task, "plan", "Make a focused change", "user")
        .unwrap();
    p.read(task, "answer.py", 1, 20).unwrap();
    p.prepare_edit(
        task,
        "answer.py",
        None,
        old,
        new,
        "replace",
        "Regression fixture",
    )
    .unwrap()
    .id
}

#[test]
fn exact_requests_and_explicit_supersession_survive_reopen() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let original = "x".repeat(32_000);
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        p.start_task("t", &original).unwrap();
        p.start_task("t", &original).unwrap();
        p.start_task("t", "Keep every boundary case").unwrap();
        let requests = p.active_requests("t").unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].body, original);
        let replacement = p
            .supersede_request(
                "t",
                requests[1].seq,
                "Keep numeric boundary cases",
                "User clarified scope",
            )
            .unwrap();
        assert_eq!(
            p.request_history("t").unwrap()[1].superseded_by,
            Some(replacement)
        );
        assert!(
            p.supersede_request("other", requests[0].seq, "bad", "wrong task")
                .is_err()
        );
    }
    let p = Project::open(data.path(), cwd.path()).unwrap();
    assert_eq!(p.request_history("t").unwrap().len(), 3);
    assert_eq!(
        p.active_requests("t").unwrap()[1].body,
        "Keep numeric boundary cases"
    );
}

#[test]
fn standard_context_retains_middle_turn_and_reports_budget_pressure() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let mut p = Project::open(data.path(), cwd.path()).unwrap();
    p.start_task("t", "Implement the small parser").unwrap();
    p.start_task("t", "Preserve the exact marker NEEDS_MIDDLE_TURN_42")
        .unwrap();
    p.start_task("t", "Continue").unwrap();
    let memory = p.memory("t", "Continue", 4096).unwrap();
    assert!(memory.contains("NEEDS_MIDDLE_TURN_42"));
    assert!(memory.chars().count() <= 4096);
    assert!(p.memory("t", "Continue", 300).is_err());
    let middle = p.active_requests("t").unwrap()[1].seq;
    p.supersede_request(
        "t",
        middle,
        "Use the new marker CORRECTED_84",
        "User changed marker",
    )
    .unwrap();
    let memory = p.memory("t", "Continue", 4096).unwrap();
    assert!(memory.contains("CORRECTED_84"));
    assert!(!memory.contains("NEEDS_MIDDLE_TURN_42"));
    assert_eq!(
        p.context_view("t").unwrap()["request_history"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
}

#[test]
fn project_lock_coordinates_different_data_roots() {
    let one = tempfile::tempdir().unwrap();
    let two = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let p = Project::open(one.path(), cwd.path()).unwrap();
    assert!(Project::open(two.path(), cwd.path()).is_err());
    drop(p);
    Project::open(two.path(), cwd.path()).unwrap();
}

#[test]
fn chmod_and_recreated_identity_are_conflicts_and_batch_undo_still_works() {
    use std::os::unix::fs::PermissionsExt;
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let path = cwd.path().join("answer.py");
    fs::write(&path, "before\n").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    let first = prepare(&p, "t", "before", "after");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(p.apply(&first, Policy::Guided).is_err());
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let first = prepare(&p, "t", "before", "after");
    p.apply(&first, Policy::Guided).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    assert!(p.undo(&first).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let second = prepare(&p, "t", "after", "third");
    p.apply(&second, Policy::Guided).unwrap();
    p.undo_task("t").unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "before\n");
    let next = prepare(&p, "t", "before", "replaced");
    fs::write(cwd.path().join("other"), "before\n").unwrap();
    fs::rename(cwd.path().join("other"), &path).unwrap();
    assert!(p.apply(&next, Policy::Guided).is_err());
}

#[test]
fn external_edit_worker() {
    let Ok(data) = std::env::var("ALT_EXTERNAL_DATA") else {
        return;
    };
    let cwd = std::env::var("ALT_EXTERNAL_CWD").unwrap();
    let id = std::env::var("ALT_EXTERNAL_CHANGE").unwrap();
    let p = Project::open(Path::new(&data), Path::new(&cwd)).unwrap();
    let result = if std::env::var("ALT_EXTERNAL_OPERATION").unwrap() == "undo" {
        p.undo(&id)
    } else {
        p.apply(&id, Policy::Guided)
    };
    let error = result.unwrap_err();
    assert!(
        format!("{error:#}").contains("external edit raced"),
        "{error:#}"
    );
    assert_eq!(p.change(&id).unwrap().status, "conflict");
    assert!(
        p.displaced_versions()
            .unwrap()
            .iter()
            .any(|r| r["sha256"] == alt_cli::project::digest(b"USER_EDIT_AT_EXCHANGE\n"))
    );
}

#[test]
fn in_place_and_rename_saves_at_atomic_boundary_are_recoverable() {
    let tools = tempfile::tempdir().unwrap();
    let library = tools.path().join("external.so");
    assert!(
        Command::new("cc")
            .args(["-shared", "-fPIC", "-O2"])
            .arg(format!(
                "{}/tests/fixtures/concurrent_edit.c",
                env!("CARGO_MANIFEST_DIR")
            ))
            .args(["-ldl", "-o"])
            .arg(&library)
            .status()
            .unwrap()
            .success()
    );
    for mode in ["write", "rename"] {
        for operation in ["apply", "undo", "delete"] {
            let data = tempfile::tempdir().unwrap();
            let cwd = tempfile::tempdir().unwrap();
            fs::write(cwd.path().join("answer.py"), "before\n").unwrap();
            let id;
            {
                let p = Project::open(data.path(), cwd.path()).unwrap();
                id = if operation == "delete" {
                    p.start_task("t", "Delete file").unwrap();
                    p.note("t", "plan", "Delete explicitly", "user").unwrap();
                    p.read("t", "answer.py", 1, 20).unwrap();
                    p.prepare_edit(
                        "t",
                        "answer.py",
                        None,
                        "",
                        "",
                        "delete",
                        "Explicit deletion",
                    )
                    .unwrap()
                    .id
                } else {
                    prepare(&p, "t", "before", "after")
                };
                if operation == "undo" {
                    p.apply(&id, Policy::Guided).unwrap();
                }
            }
            let result = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "external_edit_worker", "--nocapture"])
                .env("LD_PRELOAD", &library)
                .env("ALT_EXTERNAL_EDIT_MODE", mode)
                .env("ALT_EXTERNAL_EDIT_TARGET", "answer.py")
                .env("ALT_EXTERNAL_DATA", data.path())
                .env("ALT_EXTERNAL_CWD", cwd.path())
                .env("ALT_EXTERNAL_CHANGE", &id)
                .env("ALT_EXTERNAL_OPERATION", operation)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{mode}/{operation}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            let p = Project::open(data.path(), cwd.path()).unwrap();
            assert_eq!(p.change(&id).unwrap().status, "conflict");
            assert!(
                p.displaced_versions()
                    .unwrap()
                    .iter()
                    .any(|r| r["sha256"] == alt_cli::project::digest(b"USER_EDIT_AT_EXCHANGE\n"))
            );
        }
    }
}

#[test]
fn crash_after_exchange_retains_racing_permission_change_as_conflict() {
    use std::os::unix::fs::PermissionsExt;
    let tools = tempfile::tempdir().unwrap();
    let library = tools.path().join("external.so");
    assert!(
        Command::new("cc")
            .args(["-shared", "-fPIC", "-O2"])
            .arg(format!(
                "{}/tests/fixtures/concurrent_edit.c",
                env!("CARGO_MANIFEST_DIR")
            ))
            .args(["-ldl", "-o"])
            .arg(&library)
            .status()
            .unwrap()
            .success()
    );
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let path = cwd.path().join("answer.py");
    fs::write(&path, "before\n").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let id = {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        prepare(&p, "t", "before", "after")
    };
    let result = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "external_edit_worker", "--nocapture"])
        .env("LD_PRELOAD", &library)
        .env("ALT_EXTERNAL_EDIT_MODE", "chmod")
        .env("ALT_EXTERNAL_AFTER_RENAME_CRASH", "1")
        .env("ALT_EXTERNAL_EDIT_TARGET", "answer.py")
        .env("ALT_EXTERNAL_DATA", data.path())
        .env("ALT_EXTERNAL_CWD", cwd.path())
        .env("ALT_EXTERNAL_CHANGE", &id)
        .env("ALT_EXTERNAL_OPERATION", "apply")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(87));
    let p = Project::open(data.path(), cwd.path()).unwrap();
    assert_eq!(p.change(&id).unwrap().status, "conflict");
    let retained = p.displaced_versions().unwrap();
    assert_eq!(retained[0]["mode"], 0o600);
    assert_eq!(retained[0]["sha256"], alt_cli::project::digest(b"before\n"));
    let restore = p
        .prepare_displaced_restore(&id, "apply", retained[0]["sha256"].as_str().unwrap())
        .unwrap();
    assert_eq!(restore.after_mode, Some(0o600));
    p.apply(&restore.id, Policy::Guided).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "before\n");
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    p.undo(&restore.id).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "after\n");
}

#[test]
fn batch_undo_simulates_prior_permissions_before_any_mutation() {
    use std::os::unix::fs::PermissionsExt;
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let path = cwd.path().join("answer.py");
    fs::write(&path, "before\n").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    let one = prepare(&p, "t", "before", "first");
    p.apply(&one, Policy::Guided).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let two = prepare(&p, "t", "first", "second");
    p.apply(&two, Policy::Guided).unwrap();
    assert!(p.undo_task("t").is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), "second\n");
    assert_eq!(p.change(&two).unwrap().status, "applied");
    assert_eq!(p.change(&one).unwrap().status, "applied");
}

#[test]
fn final_status_and_identity_commit_together_before_staging_cleanup() {
    use std::os::unix::fs::PermissionsExt;
    let tools = tempfile::tempdir().unwrap();
    let library = tools.path().join("external.so");
    assert!(
        Command::new("cc")
            .args(["-shared", "-fPIC", "-O2"])
            .arg(format!(
                "{}/tests/fixtures/concurrent_edit.c",
                env!("CARGO_MANIFEST_DIR")
            ))
            .args(["-ldl", "-o"])
            .arg(&library)
            .status()
            .unwrap()
            .success()
    );
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let path = cwd.path().join("answer.py");
    fs::write(&path, "before\n").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let id = {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        prepare(&p, "t", "before", "after")
    };
    let result = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "external_edit_worker", "--nocapture"])
        .env("LD_PRELOAD", &library)
        .env("ALT_FINALIZE_CRASH", "1")
        .env("ALT_EXTERNAL_DATA", data.path())
        .env("ALT_EXTERNAL_CWD", cwd.path())
        .env("ALT_EXTERNAL_CHANGE", &id)
        .env("ALT_EXTERNAL_OPERATION", "apply")
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(88),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let p = Project::open(data.path(), cwd.path()).unwrap();
    let change = p.change(&id).unwrap();
    assert_eq!(change.status, "applied");
    assert!(change.after_identity.is_some());
    assert!(
        p.undo(&id).is_err(),
        "Same-byte external replacement must not be treated as Alt's inode"
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), "after\n");
}

#[test]
fn interrupted_recovery_keeps_project_open_with_missing_or_unsafe_paths() {
    use std::os::unix::fs::symlink;
    for kind in ["directory", "symlink", "missing-displaced"] {
        let data = tempfile::tempdir().unwrap();
        let cwd = tempfile::tempdir().unwrap();
        let path = cwd.path().join("answer.py");
        fs::write(&path, "before\n").unwrap();
        let (id, db, mut record) = {
            let p = Project::open(data.path(), cwd.path()).unwrap();
            let id = prepare(&p, "t", "before", "after");
            p.apply(&id, Policy::Guided).unwrap();
            (
                id.clone(),
                p.state.join("project.db"),
                serde_json::to_value(p.change(&id).unwrap()).unwrap(),
            )
        };
        record["status"] = serde_json::json!("applying");
        record["transition"] = serde_json::json!({"temporary":".alt-write-missing","expected_hash":record["before_hash"],"intended_hash":record["after_hash"],"expected_mode":record["mode"],"intended_identity":record["after_identity"],"operation":"apply"});
        rusqlite::Connection::open(db)
            .unwrap()
            .execute(
                "UPDATE changes SET status='applying',payload=?1 WHERE id=?2",
                rusqlite::params![record.to_string(), id],
            )
            .unwrap();
        match kind {
            "directory" => {
                fs::remove_file(&path).unwrap();
                fs::create_dir(&path).unwrap();
            }
            "symlink" => {
                fs::remove_file(&path).unwrap();
                symlink("outside.py", &path).unwrap();
            }
            _ => {}
        }
        let p = Project::open(data.path(), cwd.path()).unwrap();
        assert_eq!(p.change(&id).unwrap().status, "conflict", "{kind}");
    }
}

#[test]
fn undo_finalization_and_prior_identity_rebind_roll_back_together() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let path = cwd.path().join("answer.py");
    fs::write(&path, "before\n").unwrap();
    let (first, second, database) = {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        let first = prepare(&p, "t", "before", "first");
        p.apply(&first, Policy::Guided).unwrap();
        let second = prepare(&p, "t", "first", "second");
        p.apply(&second, Policy::Guided).unwrap();
        let database = p.state.join("project.db");
        let sql = rusqlite::Connection::open(&database).unwrap();
        sql.execute_batch(&format!("CREATE TRIGGER fail_prior_rebind BEFORE UPDATE ON changes WHEN NEW.id='{first}' BEGIN SELECT RAISE(ABORT, 'Injected prior identity write failure'); END;")).unwrap();
        assert!(p.undo(&second).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "first\n");
        assert_eq!(
            p.change(&second).unwrap().status,
            "undoing",
            "Final status must not commit without the preceding checkpoint identity"
        );
        (first, second, database)
    };
    // Recovery meets the same injected database fault. Its own final status and
    // prior-identity update must also roll back as one unit.
    assert!(Project::open(data.path(), cwd.path()).is_err());
    let sql = rusqlite::Connection::open(&database).unwrap();
    let status: String = sql
        .query_row("SELECT status FROM changes WHERE id=?1", [&second], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(status, "undoing");
    sql.execute_batch("DROP TRIGGER fail_prior_rebind").unwrap();
    drop(sql);
    let p = Project::open(data.path(), cwd.path()).unwrap();
    assert_eq!(p.change(&second).unwrap().status, "undone");
    p.undo(&first).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "before\n");
}

#[test]
fn interrupted_fifo_recovery_worker() {
    let Ok(data) = std::env::var("ALT_FIFO_RECOVERY_DATA") else {
        return;
    };
    let cwd = std::env::var("ALT_FIFO_RECOVERY_PROJECT").unwrap();
    let id = std::env::var("ALT_FIFO_RECOVERY_CHANGE").unwrap();
    let p = Project::open(Path::new(&data), Path::new(&cwd)).unwrap();
    assert_eq!(p.change(&id).unwrap().status, "conflict");
    assert!(p.read("t", "answer.py", 1, 20).is_err());
}

#[test]
fn interrupted_fifo_is_rejected_without_waiting_for_a_writer() {
    use std::os::unix::{ffi::OsStrExt, fs::FileTypeExt};
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let path = cwd.path().join("answer.py");
    fs::write(&path, "before\n").unwrap();
    let (id, database, mut record) = {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        let id = prepare(&p, "t", "before", "after");
        p.apply(&id, Policy::Guided).unwrap();
        (
            id.clone(),
            p.state.join("project.db"),
            serde_json::to_value(p.change(&id).unwrap()).unwrap(),
        )
    };
    record["status"] = serde_json::json!("applying");
    record["transition"] = serde_json::json!({"temporary":".alt-write-fifo-test","expected_hash":record["before_hash"],"intended_hash":record["after_hash"],"expected_mode":record["mode"],"intended_identity":record["after_identity"],"operation":"apply"});
    rusqlite::Connection::open(database)
        .unwrap()
        .execute(
            "UPDATE changes SET status='applying',payload=?1 WHERE id=?2",
            rusqlite::params![record.to_string(), id],
        )
        .unwrap();
    fs::remove_file(&path).unwrap();
    let fifo = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { nix::libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "interrupted_fifo_recovery_worker", "--nocapture"])
        .env("ALT_FIFO_RECOVERY_DATA", data.path())
        .env("ALT_FIFO_RECOVERY_PROJECT", cwd.path())
        .env("ALT_FIFO_RECOVERY_CHANGE", &id)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            let output = child.wait_with_output().unwrap();
            assert!(
                status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            break;
        }
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("Recovery blocked opening an external FIFO");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        fs::symlink_metadata(path).unwrap().file_type().is_fifo(),
        "Recovery must preserve the external inode"
    );
}
