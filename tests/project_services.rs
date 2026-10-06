use alt_cli::{
    project::{CheckSpec, Policy, Project},
    project_services::{Requirement, test_count},
};
use std::{
    fs,
    sync::{Arc, atomic::AtomicBool},
};
#[tokio::test]
async fn verification_requires_every_check_and_current_source_command_environment() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    fs::write(cwd.path().join("check.py"), "print('OK')\n").unwrap();
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        p.start_task("task", "Release requirements").unwrap();
        for name in ["behavior", "regression"] {
            p.set_check(&CheckSpec {
                name: name.into(),
                argv: vec!["python3".into(), "check.py".into()],
                timeout_secs: 10,
                contract: Default::default(),
            })
            .unwrap();
            p.set_requirement(&Requirement {
                name: name.into(),
                description: "Preserve behavior".into(),
                check_name: name.into(),
            })
            .unwrap();
        }
    }
    for name in ["behavior", "regression"] {
        alt_cli::sandbox::run(
            data.path().into(),
            cwd.path().into(),
            "task".into(),
            name.into(),
            Policy::Trusted,
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .unwrap();
        let p = Project::open(data.path(), cwd.path()).unwrap();
        assert_eq!(p.verification().unwrap().complete, name == "regression");
    }
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        p.set_check(&CheckSpec {
            name: "regression".into(),
            argv: vec!["false".into()],
            timeout_secs: 10,
            contract: Default::default(),
        })
        .unwrap();
        assert!(!p.verification().unwrap().complete);
        assert!(p.task("task").unwrap().check_status.contains("stale"));
    }
    fs::write(cwd.path().join("check.py"), "print('changed')\n").unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    assert!(
        p.verification()
            .unwrap()
            .requirements
            .iter()
            .all(|r| r.status.starts_with("stale"))
    );
}
#[test]
fn incremental_index_reuses_unchanged_rows_and_detects_edits_deletions_cancellation() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    fs::write(cwd.path().join("a.py"), "def old_symbol(): pass\n").unwrap();
    let mut p = Project::open(data.path(), cwd.path()).unwrap();
    assert_eq!(p.index_incremental(None).unwrap().updated, 1);
    assert_eq!(p.index_incremental(None).unwrap().updated, 0);
    fs::write(cwd.path().join("a.py"), "def new_symbol(): pass\n").unwrap();
    assert_eq!(p.search("new_symbol").unwrap().len(), 1);
    assert!(p.search("old_symbol").unwrap().is_empty());
    fs::write(cwd.path().join("b.py"), "def another(): pass\n").unwrap();
    assert!(p.index_incremental(Some(&AtomicBool::new(true))).is_err());
    assert_eq!(p.index_incremental(None).unwrap().updated, 1);
    fs::remove_file(cwd.path().join("a.py")).unwrap();
    assert_eq!(p.index_incremental(None).unwrap().removed, 1);
    assert!(p.repository_map().unwrap().to_string().contains("another"));
}

#[test]
fn discovery_enforces_file_limit_in_a_flat_directory_without_partial_index_commit() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let mut p = Project::open(data.path(), cwd.path()).unwrap();
    fs::write(cwd.path().join("keep.py"), "def preserved(): pass\n").unwrap();
    p.index_incremental(None).unwrap();
    for n in 0..50_000 {
        fs::write(cwd.path().join(format!("file-{n}")), "").unwrap();
    }
    assert!(
        p.browse().is_err(),
        "50,001 files must exceed the documented limit"
    );
    assert!(p.index_incremental(None).is_err());
    // Exactly the boundary is accepted, including when an empty directory follows.
    fs::remove_file(cwd.path().join("file-49999")).unwrap();
    fs::create_dir(cwd.path().join("empty")).unwrap();
    assert_eq!(p.browse().unwrap().len(), 50_000);
    for n in 0..49_999 {
        fs::remove_file(cwd.path().join(format!("file-{n}"))).unwrap();
    }
    let unchanged = p.index_incremental(None).unwrap();
    assert_eq!(unchanged.indexed, 1);
    assert_eq!(unchanged.updated, 0);
}
#[test]
fn pinned_requirements_and_actual_context_survive_restart() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let id = {
        let mut p = Project::open(data.path(), cwd.path()).unwrap();
        p.start_task("task", "Preserve compatibility").unwrap();
        let id = p.pin("Never rename the public API").unwrap();
        assert!(
            p.memory("task", "API", 8192)
                .unwrap()
                .contains("Never rename")
        );
        id
    };
    let p = Project::open(data.path(), cwd.path()).unwrap();
    assert!(
        p.context_view("task")
            .unwrap()
            .to_string()
            .contains("Never rename")
    );
    p.unpin(&id).unwrap();
    assert!(p.pinned().unwrap().is_empty());
}
#[test]
fn zero_tests_are_distinguished_from_unknown_output_and_mixed_rust_targets() {
    assert_eq!(test_count("Ran 0 tests in 0.01s"), Some(0));
    assert_eq!(test_count("no tests ran in 0.2s"), Some(0));
    assert_eq!(
        test_count("test result: ok. 0 passed; 0 failed;\ntest result: ok. 4 passed; 0 failed;"),
        Some(4)
    );
    assert_eq!(test_count("# tests 3\n# pass 3"), Some(3));
    assert_eq!(test_count("service healthy"), None);
}

#[tokio::test]
async fn zero_tests_after_large_output_cannot_verify_a_requirement() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    fs::write(
        cwd.path().join("check.py"),
        "print('noise\\n' * 40000)\nprint('Ran 0 tests in 0.01s')\n",
    )
    .unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    p.start_task("t", "Run real tests").unwrap();
    p.set_check(&CheckSpec {
        name: "tests".into(),
        argv: vec!["python3".into(), "check.py".into()],
        timeout_secs: 10,
        contract: Default::default(),
    })
    .unwrap();
    p.set_requirement(&Requirement {
        name: "behavior".into(),
        description: "Exercise behavior".into(),
        check_name: "tests".into(),
    })
    .unwrap();
    drop(p);
    let result = alt_cli::sandbox::run(
        data.path().into(),
        cwd.path().into(),
        "t".into(),
        "tests".into(),
        Policy::Trusted,
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .unwrap();
    assert!(result.output_truncated);
    assert_eq!(result.exit_code, Some(0));
    assert_eq!(
        result.tests_run,
        Some(0),
        "Summary must be read even after the saved-output limit"
    );
    let p = Project::open(data.path(), cwd.path()).unwrap();
    assert!(!p.verification().unwrap().complete);
    assert_eq!(
        p.verification().unwrap().requirements[0].status,
        "zero tests"
    );
    drop(p);

    // A real passing target followed by a zero-test target still counts as four.
    fs::write(cwd.path().join("check.py"), "print('test result: ok. 4 passed; 0 failed;')\nprint('noise\\n' * 40000)\nprint('test result: ok. 0 passed; 0 failed;')\n").unwrap();
    let result = alt_cli::sandbox::run(
        data.path().into(),
        cwd.path().into(),
        "t".into(),
        "tests".into(),
        Policy::Trusted,
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .unwrap();
    assert_eq!(result.tests_run, Some(4));
    let p = Project::open(data.path(), cwd.path()).unwrap();
    assert!(p.verification().unwrap().complete);
    let mut legacy = result;
    legacy.id = uuid::Uuid::new_v4().to_string();
    legacy.environment.remove("verification_contract");
    p.save_check(&legacy).unwrap();
    assert!(!p.verification().unwrap().complete);
    assert_eq!(
        p.verification().unwrap().requirements[0].status,
        "stale environment"
    );
}
#[test]
fn migrations_preserve_legacy_data_and_reject_newer_schema() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    let mut db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE original(value TEXT);INSERT INTO original VALUES('keep');")
        .unwrap();
    alt_cli::schema::migrate(&mut db, &path, 1, "CREATE TABLE newer(id TEXT);").unwrap();
    assert_eq!(
        db.query_row("SELECT value FROM original", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "keep"
    );
    let copies = fs::read_dir(dir.path().join("migrations")).unwrap().count();
    assert_eq!(copies, 1);
    assert!(alt_cli::schema::migrate(&mut db, &path, 0, "").is_err());
    let broken = alt_cli::schema::migrate(
        &mut db,
        &path,
        2,
        "CREATE TABLE partial(id TEXT);INVALID SQL;",
    );
    assert!(broken.is_err());
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(db.prepare("SELECT * FROM partial").is_err());
}

#[test]
fn syntax_and_old_decision_retrieval_keep_current_evidence_at_all_budgets() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    fs::write(
        cwd.path().join("app.py"),
        "# def fake_symbol():\ndef actual_symbol(value):\n    return value + 1\n",
    )
    .unwrap();
    let mut p = Project::open(data.path(), cwd.path()).unwrap();
    p.start_task("t", "Keep API behavior").unwrap();
    p.note(
        "t",
        "decision",
        "Cedar protocol must retain backward compatible status 204",
        "user",
    )
    .unwrap();
    for n in 0..50 {
        p.note(
            "t",
            "decision",
            &format!("Unrelated layout choice {n}"),
            "user",
        )
        .unwrap();
    }
    for budget in [4096, 8192, 16000] {
        let memory = p.memory("t", "Cedar protocol", budget).unwrap();
        assert!(memory.contains("Cedar protocol must retain"), "{memory}");
        assert!(memory.starts_with("CURRENT computed check status"));
        assert!(memory.chars().count() <= budget);
    }
    let map = p.repository_map().unwrap();
    let symbols = map["files"][0]["syntax"]["chunks"].as_array().unwrap();
    assert!(symbols.iter().any(|s| s["name"] == "actual_symbol"));
    assert!(!symbols.iter().any(|s| s["name"] == "fake_symbol"));
    assert_eq!(
        p.search("actual_symbol").unwrap()[0]["retrieval"],
        "syntax chunk"
    );
}
