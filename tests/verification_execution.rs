use alt_cli::{
    project::{CheckSpec, Policy, Project},
    project_services::Requirement,
    verification::{Contract, ExecutionInputs, Format, Kind, ReportSpec},
};
use std::{
    fs,
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};

fn configure(data: &Path, cwd: &Path, script: &str, assertion: Option<&Path>) -> CheckSpec {
    let spec = CheckSpec {
        name: "behavior".into(),
        argv: vec![
            "python3".into(),
            if assertion.is_some() {
                "{assertion}".into()
            } else {
                "-c".into()
            },
            script.into(),
        ],
        timeout_secs: 10,
        contract: if let Some(path) = assertion {
            Contract {
                kind: Kind::Tests,
                report: Some(ReportSpec {
                    format: Format::Json,
                    path: "result.json".into(),
                }),
                assertion: Some(Contract::pin(path).unwrap()),
                inputs: Default::default(),
            }
        } else {
            Contract::default()
        },
    };
    let p = Project::open(data, cwd).unwrap();
    p.start_task("t", "Independent source evidence").unwrap();
    p.set_check(&spec).unwrap();
    p.set_requirement(&Requirement {
        name: "behavior".into(),
        description: "actual behavior".into(),
        check_name: spec.name.clone(),
    })
    .unwrap();
    spec
}
async fn run(data: &Path, cwd: &Path) -> alt_cli::project::CheckResult {
    alt_cli::sandbox::run(
        data.into(),
        cwd.into(),
        "t".into(),
        "behavior".into(),
        Policy::Trusted,
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .unwrap()
}
#[tokio::test]
async fn source_mutation_and_mutate_test_restore_do_not_certify_original() {
    for restore in [false, true] {
        let data = tempfile::tempdir().unwrap();
        let cwd = tempfile::tempdir().unwrap();
        let oracle = tempfile::tempdir().unwrap();
        fs::write(cwd.path().join("answer.py"), "def answer(): return 0\n").unwrap();
        let path = oracle.path().join("assertion.py");
        fs::write(&path,format!("from pathlib import Path\nimport json,os\np=Path('answer.py')\noriginal=p.read_text()\np.write_text('def answer(): return 42\\n')\ns={{}}\nexec(p.read_text(),s)\nassert s['answer']()==42\n{}\nPath(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({{'schema':1,'complete':True,'tests':[{{'name':'answer','status':'passed'}}]}}))\n",if restore{"p.write_text(original)"}else{"pass"})).unwrap();
        configure(data.path(), cwd.path(), "", Some(&path));
        let result = run(data.path(), cwd.path()).await;
        assert_eq!(result.exit_code, Some(0));
        assert!(
            result
                .error
                .as_deref()
                .unwrap()
                .contains("changed input source")
        );
        assert!(!result.execution.observed_inputs_stable);
        assert!(!result.execution.immutable_inputs);
        if restore {
            assert_eq!(
                result.execution.transient_or_metadata_changes,
                vec!["answer.py"]
            );
        } else {
            assert_eq!(result.execution.changed_inputs, vec!["answer.py"]);
        }
        let p = Project::open(data.path(), cwd.path()).unwrap();
        assert!(!p.verification().unwrap().behavioral_acceptance);
        assert_eq!(
            fs::read_to_string(cwd.path().join("answer.py")).unwrap(),
            "def answer(): return 0\n"
        );
        assert!(
            p.state
                .join("execution")
                .join(result.id)
                .join("source.patch")
                .is_file()
        );
    }
}
#[tokio::test]
async fn normal_outputs_and_external_input_freshness_keep_their_declared_scope() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let inputs = tempfile::tempdir().unwrap();
    fs::write(cwd.path().join("answer.txt"), "42").unwrap();
    let external = inputs.path().join("flags");
    fs::write(&external, "enabled").unwrap();
    let mut spec = configure(
        data.path(),
        cwd.path(),
        "from pathlib import Path;Path('generated.txt').write_text('output');assert Path('answer.txt').read_text()=='42';print('Ran 1 test')",
        None,
    );
    spec.contract.inputs = ExecutionInputs {
        files: vec![external.clone()],
        generated: vec!["generated.txt".into()],
    };
    {
        Project::open(data.path(), cwd.path())
            .unwrap()
            .set_check(&spec)
            .unwrap();
    }
    let result = run(data.path(), cwd.path()).await;
    assert!(result.error.is_none(), "{:?}", result.error);
    assert!(result.execution.observed_inputs_stable);
    assert!(result.execution.before_manifest_sha256.is_some());
    assert_eq!(
        result.execution.before_manifest_sha256,
        result.execution.after_manifest_sha256
    );
    assert!(!result.execution.immutable_inputs);
    assert!(
        result
            .execution
            .source_scope
            .contains("do not establish immutable")
    );
    assert_eq!(result.execution.generated_paths, vec!["generated.txt"]);
    {
        assert!(
            Project::open(data.path(), cwd.path())
                .unwrap()
                .verification()
                .unwrap()
                .complete
        );
    }
    fs::write(&external, "disabled").unwrap();
    assert!(
        !Project::open(data.path(), cwd.path())
            .unwrap()
            .verification()
            .unwrap()
            .complete
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn held_project_lock_defers_attachment_and_reconciles_once_without_rerunning() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let signals = tempfile::tempdir().unwrap();
    let start = signals.path().join("start");
    let finish = signals.path().join("finish");
    let count = signals.path().join("count");
    configure(
        data.path(),
        cwd.path(),
        &format!(
            "from pathlib import Path\nimport time\nPath({:?}).write_text('started')\nPath({:?}).open('a').write('once')\nwhile not Path({:?}).exists():time.sleep(.01)\nprint('Ran 1 test')\n",
            start, count, finish
        ),
        None,
    );
    let d = data.path().to_path_buf();
    let c = cwd.path().to_path_buf();
    let task = tokio::spawn(async move { run(&d, &c).await });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !start.exists() {
        assert!(std::time::Instant::now() < deadline);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let p = Project::open(data.path(), cwd.path()).unwrap();
    fs::write(finish, "finish").unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.exit_code, Some(0));
    assert!(p.latest_check("t").unwrap().is_none());
    assert!(
        p.state
            .join("execution")
            .join(&result.id)
            .join("receipt.json")
            .is_file()
    );
    drop(p);
    for _ in 0..2 {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        assert_eq!(p.latest_check("t").unwrap().unwrap().id, result.id);
        let export = p.export().unwrap();
        assert_eq!(export["checks"].as_array().unwrap().len(), 1);
    }
    assert_eq!(fs::read_to_string(count).unwrap(), "once");
}
#[tokio::test]
async fn late_diagnostic_survives_preview_truncation_and_raw_ring_wrap() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    configure(
        data.path(),
        cwd.path(),
        "import sys;sys.stdout.write('x'*(5*1024*1024));print('LATE_REAL_DIAGNOSTIC');sys.exit(1)",
        None,
    );
    let result = run(data.path(), cwd.path()).await;
    assert_eq!(result.exit_code, Some(1));
    assert!(result.output_truncated);
    assert!(!result.output.contains("LATE_REAL_DIAGNOSTIC"));
    let p = Project::open(data.path(), cwd.path()).unwrap();
    let tail =
        alt_cli::verification::read_execution_log(&p, &result.id, "stdout", None, 4096).unwrap();
    assert!(
        tail["text"]
            .as_str()
            .unwrap()
            .contains("LATE_REAL_DIAGNOSTIC")
    );
    assert!(tail["retained_from"].as_u64().unwrap() > 0);
    assert!(
        alt_cli::verification::read_execution_log(&p, &result.id, "stdout", Some(0), 64).is_err()
    );
}
#[test]
fn inherited_feature_flags_change_freshness_without_saving_values() {
    let binary = env!("CARGO_BIN_EXE_alt");
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let invoke = |flag: &str, args: &[&str]| {
        std::process::Command::new(binary)
            .args([
                "--data-dir",
                data.path().to_str().unwrap(),
                "--access",
                "trusted",
                "task",
            ])
            .args(args)
            .env("APP_FEATURE_MODE", flag)
            .current_dir(cwd.path())
            .output()
            .unwrap()
    };
    assert!(invoke("private-enabled",&["configure-check","feature","--","python3","-c","import os;assert os.environ['APP_FEATURE_MODE']=='private-enabled';print('Ran 1 test')"]).status.success());
    assert!(
        invoke(
            "private-enabled",
            &["require", "feature", "--check", "feature"]
        )
        .status
        .success()
    );
    assert!(
        invoke("private-enabled", &["check", "feature"])
            .status
            .success()
    );
    let verify = invoke("private-disabled", &["verify"]);
    assert!(!verify.status.success());
    assert!(String::from_utf8_lossy(&verify.stdout).contains("stale environment"));
    let state = Project::open(data.path(), cwd.path()).unwrap();
    let evidence = state
        .latest_check(&state.latest_task().unwrap().unwrap())
        .unwrap()
        .unwrap();
    let serialized = serde_json::to_string(&evidence.environment).unwrap();
    assert!(!serialized.contains("private-enabled"));
    assert!(!serialized.contains("APP_FEATURE_MODE"));
}

#[tokio::test]
async fn receipt_fault_worker() {
    let Ok(data) = std::env::var("ALT_EXEC_DATA") else {
        return;
    };
    let cwd = std::env::var("ALT_EXEC_PROJECT").unwrap();
    let mode = std::env::var("ALT_EXEC_FAULT").unwrap();
    let result = alt_cli::sandbox::run(
        data.into(),
        cwd.into(),
        "t".into(),
        "behavior".into(),
        Policy::Trusted,
        Arc::new(AtomicBool::new(false)),
    )
    .await;
    if mode == "reserve-full" {
        assert!(format!("{:#}", result.unwrap_err()).contains("no command was started"));
    } else if mode == "finish-full" {
        assert!(format!("{:#}", result.unwrap_err()).contains("could not be durably saved"));
    } else {
        panic!("Expected injected process interruption");
    }
}
#[test]
fn receipt_capacity_failures_and_interrupted_commits_are_explicit_and_never_replay() {
    let tool = tempfile::tempdir().unwrap();
    let library = tool.path().join("execution-fault.so");
    assert!(
        std::process::Command::new("cc")
            .args(["-shared", "-fPIC", "-O2"])
            .arg(format!(
                "{}/tests/fixtures/execution_fault.c",
                env!("CARGO_MANIFEST_DIR")
            ))
            .args(["-ldl", "-o"])
            .arg(&library)
            .status()
            .unwrap()
            .success()
    );
    for mode in [
        "reserve-full",
        "finish-full",
        "crash-before-commit",
        "crash-after-commit",
        "crash-after-reserve",
    ] {
        let data = tempfile::tempdir().unwrap();
        let cwd = tempfile::tempdir().unwrap();
        let marker = tool.path().join(mode);
        configure(
            data.path(),
            cwd.path(),
            &format!(
                "from pathlib import Path;Path({marker:?}).open('a').write('once');print('Ran 1 test')"
            ),
            None,
        );
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "receipt_fault_worker", "--nocapture"])
            .env("LD_PRELOAD", &library)
            .env("ALT_EXEC_FAULT", mode)
            .env("ALT_EXEC_DATA", data.path())
            .env("ALT_EXEC_PROJECT", cwd.path())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(match mode {
                "crash-before-commit" => 86,
                "crash-after-commit" => 87,
                "crash-after-reserve" => 88,
                _ => 0,
            }),
            "{mode}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(marker.exists(), mode != "reserve-full");
        for _ in 0..2 {
            let p = Project::open(data.path(), cwd.path()).unwrap();
            let result = p.latest_check("t").unwrap();
            if mode == "reserve-full" {
                assert!(result.is_none());
            } else {
                let result = result.unwrap();
                if matches!(mode, "crash-after-commit" | "crash-after-reserve") {
                    assert_eq!(result.exit_code, Some(0));
                    assert!(result.error.is_none());
                } else {
                    assert!(result.error.unwrap().contains("interrupted"));
                }
            }
        }
        if marker.exists() {
            assert_eq!(fs::read_to_string(&marker).unwrap(), "once");
        }
    }
}

#[tokio::test]
async fn undeclared_new_source_and_declared_existing_input_changes_are_rejected() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let mut spec = configure(
        data.path(),
        cwd.path(),
        "from pathlib import Path;Path('new_module.py').write_text('value=42');print('Ran 1 test')",
        None,
    );
    let first = run(data.path(), cwd.path()).await;
    assert!(first.error.is_some());
    assert_eq!(first.execution.changed_inputs, vec!["new_module.py"]);
    assert!(!first.execution.patch_preview_truncated);
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        let patch = fs::read_to_string(
            p.state
                .join("execution")
                .join(&first.id)
                .join("source.patch"),
        )
        .unwrap();
        assert!(patch.contains("--- /dev/null"));
        assert!(patch.contains("+value=42"));
    }
    fs::write(cwd.path().join("answer.txt"), "wrong").unwrap();
    spec.argv = vec![
        "python3".into(),
        "-c".into(),
        "from pathlib import Path;Path('answer.txt').write_text('42');print('Ran 1 test')".into(),
    ];
    spec.contract.inputs.generated = vec!["answer.txt".into()];
    {
        Project::open(data.path(), cwd.path())
            .unwrap()
            .set_check(&spec)
            .unwrap();
    }
    let second = run(data.path(), cwd.path()).await;
    assert!(second.error.is_some());
    assert_eq!(second.execution.changed_inputs, vec!["answer.txt"]);
}
#[test]
fn cli_configures_placeholders_atomically_and_preserves_contract_on_argv_edit() {
    let binary = env!("CARGO_BIN_EXE_alt");
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(cwd.path().join("answer.txt"), "42").unwrap();
    let assertion = outside.path().join("oracle.py");
    fs::write(&assertion,"from pathlib import Path\nimport json,sys\nassert Path('answer.txt').read_text()=='42'\nPath(sys.argv[1]).write_text(json.dumps({'schema':1,'complete':True,'tests':[{'name':'answer','status':'passed'}]}))\n").unwrap();
    let invoke = |args: &[&str]| {
        std::process::Command::new(binary)
            .args([
                "--data-dir",
                data.path().to_str().unwrap(),
                "--access",
                "trusted",
                "task",
            ])
            .args(args)
            .current_dir(cwd.path())
            .output()
            .unwrap()
    };
    let configured = invoke(&[
        "configure-check",
        "behavior",
        "--kind",
        "tests",
        "--format",
        "json",
        "--report",
        "result.json",
        "--assertion",
        assertion.to_str().unwrap(),
        "--",
        "python3",
        "{assertion}",
        "{report}",
    ]);
    assert!(
        configured.status.success(),
        "{}",
        String::from_utf8_lossy(&configured.stderr)
    );
    assert!(invoke(&["check", "behavior"]).status.success());
    assert!(
        invoke(&[
            "configure-check",
            "behavior",
            "--",
            "python3",
            "{assertion}",
            "{report}",
            "--verbose"
        ])
        .status
        .success()
    );
    let spec = {
        Project::open(data.path(), cwd.path())
            .unwrap()
            .checks()
            .unwrap()
            .pop()
            .unwrap()
    };
    assert_eq!(spec.contract.kind, Kind::Tests);
    assert!(spec.contract.assertion.is_some());
    assert_eq!(spec.contract.report.unwrap().path, "result.json");
    assert!(invoke(&["check", "behavior"]).status.success());
}
