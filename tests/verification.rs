use alt_cli::{
    project::{CheckSpec, Policy, Project},
    project_services::Requirement,
    verification::{self, Contract, Format, Kind, ReportSpec},
};
use std::{
    fs,
    sync::{Arc, atomic::AtomicBool},
};
#[test]
fn structured_reports_reject_missing_incomplete_contradictory_and_fabricated_totals() {
    for (format, bytes) in [
        (Format::Json, r#"{"schema":1,"complete":false,"tests":[]}"#),
        (Format::Json, r#"{"schema":1,"complete":true,"passed":5}"#),
        (
            Format::Json,
            r#"{"schema":1,"complete":true,"tests":[{"name":"a","status":"passed"},{"name":"a","status":"passed"}]}"#,
        ),
        (
            Format::Junit,
            "<testsuite tests='2'><testcase name='one'/></testsuite>",
        ),
        (
            Format::Junit,
            "<testsuite tests='1' failures='0'><testcase name='one'><failure/></testcase></testsuite>",
        ),
        (
            Format::Junit,
            "<!DOCTYPE testsuite [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><testsuite/>",
        ),
        (Format::Tap, "TAP version 13\n1..2\nok 1 - one\n"),
        (Format::Tap, "1..2\nok 1\nok 1\n"),
        (Format::Tap, "1..1\nBail out!\n"),
    ] {
        assert!(
            verification::parse(format, bytes.as_bytes()).is_err(),
            "{bytes}"
        );
    }
    for (format, bytes) in [
        (
            Format::Json,
            r#"{"schema":1,"complete":true,"tests":[{"name":"one","status":"passed"},{"name":"two","status":"failed"},{"name":"three","status":"skipped"}]}"#,
        ),
        (
            Format::Junit,
            "<testsuites tests='3'><testsuite name='suite' tests='3' failures='1' skipped='1'><testcase name='one'/><testcase name='two'><failure/></testcase><testcase name='three'><skipped/></testcase></testsuite></testsuites>",
        ),
        (
            Format::Tap,
            "TAP version 13\n1..3\nok 1 - one\nnot ok 2 - two\nok 3 - three # SKIP unavailable\n",
        ),
    ] {
        let r = verification::parse(format, bytes.as_bytes()).unwrap();
        assert_eq!((r.passed, r.failed, r.skipped, r.executed), (1, 1, 1, 2));
    }
    assert_eq!(
        verification::parse(Format::Tap, b"1..0 # SKIP no tests\n")
            .unwrap()
            .executed,
        0
    );
}
#[tokio::test]
async fn independent_check_contract_rejects_fake_output_and_stales_on_assertion_or_dependency_changes()
 {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let assertions = tempfile::tempdir().unwrap();
    fs::write(cwd.path().join("answer.txt"), "42").unwrap();
    let assertion = assertions.path().join("behavior.py");
    fs::write(&assertion,"import json,os\nfrom pathlib import Path\nstatus='passed' if Path('answer.txt').read_text()=='42' else 'failed'\nPath(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'answer','status':status}]}))\n").unwrap();
    let mut spec = CheckSpec {
        name: "behavior".into(),
        argv: vec!["python3".into(), "{assertion}".into()],
        timeout_secs: 10,
        contract: Contract {
            kind: Kind::Tests,
            inputs: Default::default(),
            report: Some(ReportSpec {
                format: Format::Json,
                path: "result.json".into(),
            }),
            assertion: Some(Contract::pin(&assertion).unwrap()),
        },
    };
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        p.start_task("t", "Answer correctly").unwrap();
        p.set_check(&spec).unwrap();
        p.set_requirement(&Requirement {
            name: "answer".into(),
            description: "Produces 42".into(),
            check_name: spec.name.clone(),
        })
        .unwrap();
    }
    let run = || {
        alt_cli::sandbox::run(
            data.path().into(),
            cwd.path().into(),
            "t".into(),
            "behavior".into(),
            Policy::Trusted,
            Arc::new(AtomicBool::new(false)),
        )
    };
    let r = run().await.unwrap();
    assert!(r.error.is_none(), "{:?}", r.error);
    assert_eq!(r.tests_run, Some(1));
    assert!(r.structured.unwrap().assertion_verified);
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        let v = p.verification().unwrap();
        assert!(v.complete && v.behavioral_acceptance);
        assert!(
            p.state
                .join("check-reports")
                .join(format!("{}.report", r.id))
                .is_file()
        );
    }
    let dependencies = cwd.path().join(".venv/lib/site-packages/pkg");
    fs::create_dir_all(&dependencies).unwrap();
    fs::write(dependencies.join("__init__.py"), "VERSION=1").unwrap();
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        assert!(!p.verification().unwrap().complete);
    }
    run().await.unwrap();
    fs::write(dependencies.join("__init__.py"), "VERSION=2").unwrap();
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        assert!(!p.verification().unwrap().complete);
    }
    fs::write(&assertion, "print('forged assertion')").unwrap();
    let r = run().await.unwrap();
    assert!(r.error.unwrap().contains("assertion changed"));
    assert_eq!(r.exit_code, None);
    spec.contract.assertion = None;
    spec.argv = vec![
        "python3".into(),
        "-c".into(),
        "print('Ran 99 tests in 0s')".into(),
    ];
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        p.set_check(&spec).unwrap();
    }
    // A report copied from the project before execution must not be reused.
    fs::write(
        cwd.path().join("result.json"),
        r#"{"schema":1,"complete":true,"tests":[{"name":"fake","status":"passed"}]}"#,
    )
    .unwrap();
    let r = run().await.unwrap();
    assert!(r.error.unwrap().contains("Structured evidence incomplete"));
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        assert!(!p.verification().unwrap().complete);
    }
    spec.contract = Contract {
        kind: Kind::Build,
        ..Default::default()
    };
    spec.argv = vec![
        "python3".into(),
        "-c".into(),
        "print('Ran 0 tests in 0s')".into(),
    ];
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        p.set_check(&spec).unwrap();
    }
    assert!(run().await.unwrap().error.is_none());
    let p = Project::open(data.path(), cwd.path()).unwrap();
    let v = p.verification().unwrap();
    assert!(v.complete);
    assert!(!v.behavioral_acceptance);
    assert!(
        v.scope
            .contains("independent behavioral acceptance is not established")
    );
}
#[test]
fn newer_state_is_rejected_before_any_schema_or_journal_mutation() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let state = {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        p.state.clone()
    };
    for path in [state.join("project.db"), data.path().join("sessions.db")] {
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA user_version=99; CREATE TABLE future_only(value TEXT);").unwrap();
        drop(db);
        let before = fs::read(&path).unwrap();
        if path.file_name().unwrap() == "project.db" {
            assert!(Project::open(data.path(), cwd.path()).is_err());
        } else {
            assert!(alt_cli::store::Store::open(data.path()).is_err());
        }
        assert_eq!(before, fs::read(&path).unwrap());
    }
}

#[test]
fn cli_can_convert_an_existing_script_command_into_a_pinned_contract() {
    let data = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let script = outside.path().join("acceptance.py");
    fs::write(&script, "pass\n").unwrap();
    let invoke = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_alt"))
            .arg("--data-dir")
            .arg(data.path())
            .args(args)
            .current_dir(project.path())
            .output()
            .unwrap()
    };
    assert!(
        invoke(&[
            "task",
            "configure-check",
            "acceptance",
            "--",
            "python3",
            script.to_str().unwrap()
        ])
        .status
        .success()
    );
    let result = invoke(&[
        "task",
        "contract",
        "acceptance",
        "--kind",
        "tests",
        "--format",
        "json",
        "--report",
        "result.json",
        "--assertion",
        script.to_str().unwrap(),
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["argv"][1], "{assertion}");
    assert!(value["contract"]["assertion"]["sha256"].is_string());
}

#[test]
fn dependency_symlinks_track_target_changes_without_following_cycles_forever() {
    let project = tempfile::tempdir().unwrap();
    let library = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("node_modules")).unwrap();
    fs::write(library.path().join("package.js"), "before").unwrap();
    std::os::unix::fs::symlink(library.path(), project.path().join("node_modules/pkg")).unwrap();
    std::os::unix::fs::symlink(
        project.path().join("node_modules"),
        library.path().join("cycle"),
    )
    .unwrap();
    let before = verification::dependencies(project.path()).unwrap();
    fs::write(library.path().join("package.js"), "after!").unwrap();
    assert_ne!(verification::dependencies(project.path()).unwrap(), before);
}
