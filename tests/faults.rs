use alt_cli::project::{Policy, Project};
use std::{fs, process::Command};
#[test]
fn fault_worker() {
    let Ok(data) = std::env::var("ALT_FAULT_DATA") else {
        return;
    };
    let cwd = std::env::var("ALT_FAULT_PROJECT").unwrap();
    let id = std::env::var("ALT_FAULT_CHANGE").unwrap();
    let p = Project::open(std::path::Path::new(&data), std::path::Path::new(&cwd)).unwrap();
    let result = if std::env::var("ALT_FAULT_OPERATION").unwrap() == "apply" {
        p.apply(&id, Policy::Guided)
    } else {
        p.undo(&id)
    };
    if std::env::var("ALT_FAULT_MODE").unwrap() == "enospc" {
        let error = result.unwrap_err();
        assert!(error.to_string().contains("No space left"), "{error:#}");
    } else {
        panic!("Crash injector did not activate");
    }
}
#[test]
fn disk_full_and_abrupt_apply_undo_recover_without_replay_or_partial_source() {
    let tooling = tempfile::tempdir().unwrap();
    let library = tooling.path().join("fault.so");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2"])
        .arg(format!(
            "{}/tests/fixtures/fault_io.c",
            env!("CARGO_MANIFEST_DIR")
        ))
        .args(["-ldl", "-o"])
        .arg(&library)
        .status()
        .unwrap();
    assert!(status.success());
    for operation in ["apply", "undo"] {
        for mode in ["enospc", "crash-before-rename", "crash-after-rename"] {
            let data = tempfile::tempdir().unwrap();
            let cwd = tempfile::tempdir().unwrap();
            fs::write(cwd.path().join("app.py"), "before\n").unwrap();
            let id;
            {
                let p = Project::open(data.path(), cwd.path()).unwrap();
                p.start_task("t", "Fault recovery").unwrap();
                p.note("t", "plan", "Update source", "user").unwrap();
                p.read("t", "app.py", 1, 20).unwrap();
                let c = p
                    .prepare_edit(
                        "t",
                        "app.py",
                        None,
                        "before",
                        "after",
                        "replace",
                        "Fixture change",
                    )
                    .unwrap();
                id = c.id;
                if operation == "undo" {
                    p.apply(&id, Policy::Guided).unwrap();
                }
            }
            let result = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "fault_worker", "--nocapture"])
                .env("LD_PRELOAD", &library)
                .env("ALT_FAULT_DATA", data.path())
                .env("ALT_FAULT_PROJECT", cwd.path())
                .env("ALT_FAULT_CHANGE", &id)
                .env("ALT_FAULT_OPERATION", operation)
                .env("ALT_FAULT_MODE", mode)
                .output()
                .unwrap();
            assert_eq!(
                result.status.code(),
                Some(match mode {
                    "enospc" => 0,
                    "crash-before-rename" => 86,
                    _ => 87,
                }),
                "{operation} {mode}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            let completed = mode == "crash-after-rename";
            let expected = match (operation, completed) {
                ("apply", true) | ("undo", false) => "after\n",
                _ => "before\n",
            };
            assert_eq!(
                fs::read_to_string(cwd.path().join("app.py")).unwrap(),
                expected
            );
            {
                let p = Project::open(data.path(), cwd.path()).unwrap();
                let status = p.change(&id).unwrap().status;
                assert_eq!(
                    status,
                    match (operation, completed) {
                        ("apply", false) => "proposed",
                        ("undo", true) => "undone",
                        _ => "applied",
                    }
                );
            }
            // A second open reconciles records only; never automatically repeats the action.
            drop(Project::open(data.path(), cwd.path()).unwrap());
            assert_eq!(
                fs::read_to_string(cwd.path().join("app.py")).unwrap(),
                expected
            );
        }
    }
}
