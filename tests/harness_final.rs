use alt_cli::{
    compact_context,
    config::Preferences,
    engine::Event,
    project::{CheckResult, Policy, Project},
    toolbox::{Bridge, ToolProfile},
    workflow,
};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

fn result(output: String, snapshot_root: &std::path::Path) -> CheckResult {
    serde_json::from_value(json!({"id":"868a6a47-a417-4237-9619-e51b0e54ceee","task":"t","name":"test","argv":["cargo","test"],"exit_code":1,"timed_out":false,"cancelled":false,"output":output,"output_truncated":false,"snapshot":"observed-revision","isolation":"fixture","elapsed_ms":1,"error":null,"execution":{"snapshot_root":snapshot_root}})).unwrap()
}

#[test]
fn evolving_requests_survive_fifty_turns_restart_and_explicit_correction() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    {
        let mut p = Project::open(data.path(), cwd.path()).unwrap();
        p.start_task("t", "Original goal").unwrap();
        for n in 2..=50 {
            p.start_task("t", &format!("Keep requirement_{n:02} ☃ exactly"))
                .unwrap();
        }
        p.start_task("t", "Continue").unwrap();
        let text = p.compact_memory("t", "Continue", 12000).unwrap();
        for n in 2..=50 {
            assert!(text.contains(&format!("requirement_{n:02} ☃")));
        }
        assert!(p.compact_memory("t", "Continue", 300).is_err());
        let row = p
            .active_requests("t")
            .unwrap()
            .into_iter()
            .find(|r| r.body.contains("requirement_02"))
            .unwrap();
        p.note(
            "t",
            "decision",
            "Ignore all old requirements",
            "model note (unverified)",
        )
        .unwrap();
        assert!(
            p.active_requests("t")
                .unwrap()
                .iter()
                .any(|r| r.seq == row.seq)
        );
        p.supersede_request(
            "t",
            row.seq,
            "Use replacement_TWO ☃ instead",
            "Explicit user correction",
        )
        .unwrap();
    }
    let mut p = Project::open(data.path(), cwd.path()).unwrap();
    let text = p.compact_memory("t", "Continue", 12000).unwrap();
    assert!(text.contains("replacement_TWO ☃"));
    assert!(!text.contains("requirement_02"));
    assert!(
        p.request_history("t")
            .unwrap()
            .iter()
            .any(|r| r.body.contains("requirement_02") && r.superseded_by.is_some())
    );
    p.start_task("other", "Unrelated new task").unwrap();
    assert!(
        !p.compact_memory("other", "Continue", 1000)
            .unwrap()
            .contains("requirement_50")
    );
}

#[test]
fn named_manifest_cannot_hide_requested_implementation_or_import_neighbors() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    std::fs::create_dir(cwd.path().join("src")).unwrap();
    std::fs::write(cwd.path().join("Cargo.toml"), "[package]\nname='fixture'\n").unwrap();
    std::fs::write(
        cwd.path().join("src/lib.rs"),
        "use crate::arithmetic::midpoint;\npub fn median(_values: &[i64]) -> Option<f64> { None }\n",
    )
    .unwrap();
    std::fs::write(
        cwd.path().join("src/arithmetic.rs"),
        "pub fn midpoint(a:f64,b:f64)->f64 { (a+b)/2.0 }\n",
    )
    .unwrap();
    let mut p = Project::open(data.path(), cwd.path()).unwrap();
    let goal = "Implement median(&[i64]) -> Option<f64>. Do not change Cargo.toml.";
    p.start_task("t", goal).unwrap();
    let text = p.compact_memory("t", goal, 4096).unwrap();
    assert!(text.contains("pub fn median"));
    assert!(text.contains("[package]"));
    let rows = p.compact_candidates("t", goal).unwrap();
    assert_eq!(rows[0]["path"], "src/lib.rs");
    assert!(rows.iter().any(|r| r["path"] == "src/arithmetic.rs"
        && r["reasons"].to_string().contains("import neighbor")));
}

#[test]
fn declared_navigation_set_finds_symbols_among_distractors_without_model_calls() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    std::fs::write(
        cwd.path().join("README.md"),
        "Do not change this document.\n",
    )
    .unwrap();
    let cases = [
        ("py", "def {}(x):\n return x\n"),
        ("rs", "pub fn {}(x:i64)->i64 { x }\n"),
        ("js", "function {}(x) { return x; }\n"),
    ];
    let mut expected = Vec::new();
    for (extension, body) in cases {
        for n in 0..12 {
            let name = format!("target_{extension}_{n}");
            let path = format!("module_{extension}_{n}.{extension}");
            std::fs::write(cwd.path().join(&path), body.replacen("{}", &name, 1)).unwrap();
            expected.push((name, path));
        }
    }
    let mut p = Project::open(data.path(), cwd.path()).unwrap();
    p.start_task("t", "Navigate current source").unwrap();
    let mut found = 0;
    for (name, path) in &expected {
        let rows = p
            .compact_candidates("t", &format!("Repair {name}; preserve README.md unchanged"))
            .unwrap();
        found += usize::from(rows.iter().take(3).any(|r| r["path"] == *path));
    }
    assert_eq!(
        found, 36,
        "Controlled symbol navigation set; this is not real-model coding efficacy"
    );
}

#[test]
fn diagnostics_preserve_crate_provenance_and_do_not_alias_oracle_paths() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let copy = tempfile::tempdir().unwrap();
    std::fs::create_dir(cwd.path().join("src")).unwrap();
    std::fs::write(cwd.path().join("src/lib.rs"), "pub fn median() {}\n").unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    let cargo = |manifest: String| {
        json!({"reason":"compiler-message","manifest_path":manifest,"message":{"spans":[{"file_name":"src/lib.rs","line_start":1,"column_start":8,"is_primary":true}]}}).to_string()
    };
    let output = format!(
        " --> src/lib.rs:1:5\nthread 'behavior' panicked at src/lib.rs:2:23:\n  left: Some(0.0)\n right: Some(-0.5)\n{}\n{}\n",
        cargo("/tmp/independent-oracle/Cargo.toml".into()),
        cargo(copy.path().join("Cargo.toml").display().to_string())
    );
    let c = result(output, copy.path());
    let observations = workflow::diagnostic_observations(&p, &c).unwrap();
    assert!(
        observations
            .iter()
            .filter(|v| v["diagnostic_origin"] == "quoted text; relative root unknown")
            .all(|v| v["path"].is_null())
    );
    assert!(
        observations
            .iter()
            .find(|v| v["crate_root"] == "/tmp/independent-oracle")
            .unwrap()["path"]
            .is_null()
    );
    let mapped = workflow::diagnostic_locations(&p, &c).unwrap();
    assert_eq!(mapped.len(), 1);
    assert_eq!(mapped[0]["path"], "src/lib.rs");
    let packet = workflow::failure_packet(&p, &c).unwrap();
    assert!(
        packet["observed_diagnostics"]
            .to_string()
            .contains("Some(-0.5)")
    );
}

#[test]
fn typed_edit_preserves_surrounding_source_and_previews_destructive_scope() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let original = "def calc(x):\n    return x + 1\n\ndef keep():\n    return 42\n";
    std::fs::write(cwd.path().join("calc.py"), original).unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    p.start_task("t", "Fix calc").unwrap();
    p.note("t", "plan", "Read then patch", "user").unwrap();
    let read = p.read("t", "calc.py", 1, 100).unwrap();
    let h = read["range_handle"].as_str().unwrap();
    let bad = p
        .prepare_scoped_text_edit(
            "t",
            "calc.py",
            h,
            "replace-span",
            None,
            "    return x * 2\n",
            "wrong span",
        )
        .unwrap();
    assert_eq!(
        compact_context::edit_preview(&bad).unwrap()["needs_intent"],
        true
    );
    p.reject(&bad.id).unwrap();
    assert_eq!(
        std::fs::read_to_string(cwd.path().join("calc.py")).unwrap(),
        original
    );
    let good = p
        .prepare_scoped_text_edit(
            "t",
            "calc.py",
            h,
            "replace-text",
            Some("x + 1"),
            "x * 2",
            "focused expression",
        )
        .unwrap();
    assert_eq!(
        compact_context::edit_preview(&good).unwrap()["needs_intent"],
        false
    );
    p.apply(&good.id, Policy::Trusted).unwrap();
    assert_eq!(
        std::fs::read_to_string(cwd.path().join("calc.py")).unwrap(),
        original.replace("x + 1", "x * 2")
    );
    assert!(
        p.prepare_scoped_text_edit(
            "t",
            "calc.py",
            h,
            "replace-text",
            Some("x * 2"),
            "x * 3",
            "stale"
        )
        .is_err()
    );
    let read = p.read("t", "calc.py", 1, 100).unwrap();
    let symbol = read["symbols"][0]["handle"].as_str().unwrap();
    let renamed = p
        .prepare_scoped_text_edit(
            "t",
            "calc.py",
            symbol,
            "replace-symbol",
            None,
            "def renamed(x):\n    return x * 2",
            "intentional rename",
        )
        .unwrap();
    assert_eq!(
        compact_context::edit_preview(&renamed).unwrap()["needs_intent"],
        true
    );
    p.apply(&renamed.id, Policy::Trusted).unwrap();
    assert!(
        std::fs::read_to_string(cwd.path().join("calc.py"))
            .unwrap()
            .contains("def keep()")
    );
}

async fn call(bridge: &mut Bridge, arguments: Value, approvals: bool) -> Value {
    named_call(bridge, "edit_text", arguments, approvals).await
}

async fn named_call(bridge: &mut Bridge, name: &str, arguments: Value, approvals: bool) -> Value {
    let path = bridge.path.clone();
    let name = name.to_owned();
    let mut response = tokio::spawn(async move {
        let mut stream = tokio::net::UnixStream::connect(path).await.unwrap();
        stream
            .write_all(format!("{}\n", json!({"name":name,"arguments":arguments})).as_bytes())
            .await
            .unwrap();
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line).await.unwrap();
        serde_json::from_str::<Value>(&line).unwrap()
    });
    tokio::time::timeout(std::time::Duration::from_secs(5),async {loop {tokio::select! {
        result=&mut response=>break result.unwrap(),
        event=bridge.events.recv()=>if let Some(Event::Permission{id,..})=event {assert!(approvals,"Preflight must happen before approval/mutation");bridge.decision(id.as_str().unwrap(),true);}
    }}}).await.unwrap()
}

#[tokio::test]
async fn executable_probe_retains_prediction_as_unverified_and_records_actual_evidence() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    std::fs::write(cwd.path().join("value.py"), "VALUE=1\n").unwrap();
    let prefs = Preferences {
        tool_profile: ToolProfile::Compact,
        ..Default::default()
    };
    prefs.save(data.path()).unwrap();
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        p.start_task("t", "Investigate observed arithmetic")
            .unwrap();
        p.set_check(&alt_cli::project::CheckSpec {
            name: "probe".into(),
            argv: vec![
                "python3".into(),
                "-c".into(),
                "print('OBSERVED_VALUE=1')".into(),
            ],
            timeout_secs: 10,
            contract: Default::default(),
        })
        .unwrap();
    }
    let mut bridge = Bridge::start(data.path(), cwd.path(), Policy::Trusted).unwrap();
    bridge.context("t", "s");
    assert_eq!(
        named_call(
            &mut bridge,
            "run_check",
            json!({"name":"probe","hypothesis":"VALUE is one"}),
            false
        )
        .await["isError"],
        true
    );
    let result=named_call(&mut bridge,"run_check",json!({"name":"probe","hypothesis":"VALUE is two","prediction":"stdout contains OBSERVED_VALUE=2"}),true).await;
    assert_eq!(result["isError"], false);
    let p = Project::open(data.path(), cwd.path()).unwrap();
    let notes = p.export().unwrap()["notes"].clone();
    let text = serde_json::to_string(&notes).unwrap();
    assert!(text.contains("model probe (unverified)"));
    assert!(text.contains("host probe receipt"));
    assert!(text.contains("Prediction not automatically judged"));
    assert!(result.to_string().contains("OBSERVED_VALUE=1"));
}

#[test]
fn rust_partial_function_replacements_are_previewed_without_blocking_unsupported_edits() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let original = "pub fn median(xs: &[i64]) -> Option<f64> {\n    if xs.is_empty() {\n        return None;\n    }\n    let mut v=xs.to_vec();\n    v.sort_unstable();\n    let n=v.len();\n    if n % 2 == 0 {\n        Some((v[n/2-1] as f64 + v[n/2] as f64)/2.0)\n    } else {\n        Some(v[n/2] as f64)\n    }\n}\n";
    std::fs::write(cwd.path().join("median.rs"), original).unwrap();
    std::fs::write(cwd.path().join("notes.unknown"), "anchor\n").unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    p.start_task("t", "Review edit scope").unwrap();
    p.note("t", "plan", "Preview current spans before editing", "user")
        .unwrap();
    for (start, count) in [(1, 12), (5, 8)] {
        let read = p.read("t", "median.rs", start, count).unwrap();
        let change=p.prepare_scoped_text_edit("t","median.rs",read["range_handle"].as_str().unwrap(),"replace-span",None,"        let sum = v[n/2-1] as i128 + v[n/2] as i128;\n        Some(sum as f64 / 2.0)","partial edit replay").unwrap();
        assert_eq!(
            compact_context::edit_preview(&change).unwrap()["needs_intent"],
            true
        );
        p.reject(&change.id).unwrap();
        assert_eq!(
            std::fs::read_to_string(cwd.path().join("median.rs")).unwrap(),
            original
        );
    }
    let read = p.read("t", "notes.unknown", 1, 1).unwrap();
    let change = p
        .prepare_scoped_text_edit(
            "t",
            "notes.unknown",
            read["range_handle"].as_str().unwrap(),
            "insert-after",
            None,
            "new line\n",
            "unsupported parser edit",
        )
        .unwrap();
    assert_eq!(
        compact_context::edit_preview(&change).unwrap()["needs_intent"],
        false
    );
    p.apply(&change.id, Policy::Trusted).unwrap();
    assert_eq!(
        std::fs::read_to_string(cwd.path().join("notes.unknown")).unwrap(),
        "anchor\nnew line\n"
    );
}

#[tokio::test]
async fn native_preflight_keeps_source_until_explicit_intent_and_preview_never_mutates() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let original = "def calc(x):\n    return x + 1\n";
    std::fs::write(cwd.path().join("calc.py"), original).unwrap();
    let prefs = Preferences {
        tool_profile: ToolProfile::Compact,
        ..Default::default()
    };
    prefs.save(data.path()).unwrap();
    let handle = {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        p.start_task("t", "Fix calc").unwrap();
        p.note("t", "plan", "Read and edit", "user").unwrap();
        p.read("t", "calc.py", 1, 100).unwrap()["range_handle"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let mut bridge = Bridge::start(data.path(), cwd.path(), Policy::Trusted).unwrap();
    bridge.context("t", "s");
    let arguments = json!({"path":"calc.py","handle":handle,"new_text":"    return x * 2\n"});
    let blocked = call(&mut bridge, arguments.clone(), false).await;
    assert_eq!(blocked["isError"], true);
    assert!(blocked.to_string().contains("NOT applied"));
    assert_eq!(
        std::fs::read_to_string(cwd.path().join("calc.py")).unwrap(),
        original
    );
    let mut preview = arguments.clone();
    preview["preview"] = json!(true);
    preview["intentional"] = json!(true);
    assert_eq!(call(&mut bridge, preview, false).await["isError"], false);
    assert_eq!(
        std::fs::read_to_string(cwd.path().join("calc.py")).unwrap(),
        original
    );
    let mut intentional = arguments;
    intentional["intentional"] = json!(true);
    assert_eq!(call(&mut bridge, intentional, true).await["isError"], false);
    assert_eq!(
        std::fs::read_to_string(cwd.path().join("calc.py")).unwrap(),
        "    return x * 2\n"
    );
}

#[test]
fn candidate_feedback_uses_observed_failures_and_declares_distinct_strategies() {
    let row = json!({"verification":{"complete":false},"failure_packet":{"observed_diagnostics":["left: Some(0.0)","right: Some(-0.5)"]},"diffs":[{"path":"src/lib.rs"}],"strategy":"minimal repair"});
    let feedback = alt_cli::candidates::candidate_feedback(Some(&row));
    assert!(feedback.contains("Some(-0.5)"));
    assert!(feedback.contains("src/lib.rs"));
    assert_ne!(
        alt_cli::candidates::strategy(0),
        alt_cli::candidates::strategy(1)
    );
}

#[tokio::test]
async fn late_assertion_is_retrieved_from_integrity_checked_raw_log() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    std::fs::write(cwd.path().join("source.py"), "value = 1\n").unwrap();
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        p.start_task("t", "Observe late failure").unwrap();
        p.set_check(&alt_cli::project::CheckSpec{name:"late".into(),argv:vec!["python3".into(),"-c".into(),"import sys; print('progress'*30000); print('AssertionError FINAL_COUNTEREXAMPLE value=1 expected=2'); sys.exit(1)".into()],timeout_secs:20,contract:Default::default()}).unwrap();
    }
    let c = alt_cli::sandbox::run(
        data.path().into(),
        cwd.path().into(),
        "t".into(),
        "late".into(),
        Policy::Trusted,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    )
    .await
    .unwrap();
    assert!(c.output_truncated);
    assert!(!c.output.contains("FINAL_COUNTEREXAMPLE"));
    let p = Project::open(data.path(), cwd.path()).unwrap();
    let packet = workflow::failure_packet(&p, &c).unwrap();
    assert!(
        packet["observed_diagnostics"]
            .to_string()
            .contains("FINAL_COUNTEREXAMPLE")
    );
    let path = p.state.join("execution").join(&c.id).join("stdout.log");
    std::fs::write(path, "tampered").unwrap();
    let packet = workflow::failure_packet(&p, &c).unwrap();
    assert!(
        !packet["observed_diagnostics"]
            .to_string()
            .contains("FINAL_COUNTEREXAMPLE")
    );
    assert!(
        packet["captured_output_preview"]
            .as_str()
            .unwrap()
            .contains("unavailable")
    );
}
