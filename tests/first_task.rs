use alt_cli::{
    practice,
    project::{Policy, Project},
    toolbox::{self, ToolProfile},
};
use std::{
    fs,
    sync::{Arc, atomic::AtomicBool},
};

#[tokio::test]
async fn practice_repair_verification_restart_undo_and_new_lesson() {
    let data = tempfile::tempdir().unwrap();
    let lesson = practice::create(data.path()).unwrap();
    let run = || {
        alt_cli::sandbox::run(
            data.path().into(),
            lesson.project.clone(),
            lesson.task.clone(),
            "Practice behavior".into(),
            Policy::Trusted,
            Arc::new(AtomicBool::new(false)),
        )
    };
    let broken = run().await.unwrap();
    assert_eq!(broken.exit_code, Some(1));
    assert_eq!(broken.tests_run, Some(4));
    assert!(
        !Project::open(data.path(), &lesson.project)
            .unwrap()
            .verification()
            .unwrap()
            .complete
    );
    let change = {
        let p = Project::open(data.path(), &lesson.project).unwrap();
        p.read(&lesson.task, "greeting.py", 1, 100).unwrap();
        let c = p
            .prepare_edit(
                &lesson.task,
                "greeting.py",
                None,
                "name +",
                "(name.strip() or 'friend') +",
                "replace",
                "Trim names and provide a default",
            )
            .unwrap();
        p.apply(&c.id, Policy::Trusted).unwrap();
        c.id
    };
    let fixed = run().await.unwrap();
    assert_eq!(fixed.exit_code, Some(0));
    assert!(fixed.structured.unwrap().assertion_verified);
    {
        // Reopening is a real persisted-state check, not a badge assertion.
        let p = Project::open(data.path(), &lesson.project).unwrap();
        assert!(p.verification().unwrap().behavioral_acceptance);
        p.undo(&change).unwrap();
        assert!(!p.verification().unwrap().complete);
    }
    assert_eq!(run().await.unwrap().exit_code, Some(1));
    let another = practice::create(data.path()).unwrap();
    assert_ne!(another.project, lesson.project);
    assert_eq!(
        fs::read_to_string(lesson.project.join("greeting.py")).unwrap(),
        practice::SEED
    );
}

#[test]
fn tool_guidance_matches_focus_and_does_not_offer_unavailable_execution() {
    let data = tempfile::tempdir().unwrap();
    let lesson = practice::create(data.path()).unwrap();
    let checks = Project::open(data.path(), &lesson.project)
        .unwrap()
        .checks()
        .unwrap();
    let coding = toolbox::guidance(ToolProfile::Coding, Policy::Trusted, &checks);
    assert!(coding.contains("Practice behavior"));
    assert!(coding.contains("Terminal execution is unavailable"));
    assert!(!coding.contains("terminal can execute"));
    assert!(
        toolbox::guidance(ToolProfile::Coding, Policy::Trusted, &[]).contains("No named checks")
    );
    assert!(
        toolbox::guidance(ToolProfile::All, Policy::Trusted, &[])
            .contains("terminal can execute arbitrary commands")
    );
    assert!(
        !toolbox::guidance(ToolProfile::All, Policy::ReviewOnly, &checks)
            .contains("Execute tests through")
    );
    assert!(
        !toolbox::guidance(ToolProfile::Inspect, Policy::Trusted, &checks)
            .contains("Configured run_check names")
    );
}

#[test]
fn suggestions_follow_declared_runners_and_do_not_invent_tests() {
    let cwd = tempfile::tempdir().unwrap();
    fs::write(
        cwd.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();
    fs::create_dir(cwd.path().join("tests")).unwrap();
    fs::write(cwd.path().join("tests/integration.rs"), "").unwrap();
    assert!(alt_cli::sandbox::suggested(cwd.path()).is_empty());
    fs::write(
        cwd.path().join("package.json"),
        r#"{"packageManager":"pnpm@10","scripts":{"test":"node --test"}}"#,
    )
    .unwrap();
    assert_eq!(
        alt_cli::sandbox::suggested(cwd.path())[0].argv,
        ["pnpm", "test"]
    );
    fs::write(
        cwd.path().join("pyproject.toml"),
        "[tool.pytest.ini_options]\n",
    )
    .unwrap();
    let suggestions = alt_cli::sandbox::suggested(cwd.path());
    let py = suggestions
        .iter()
        .find(|s| s.name == "Python pytest")
        .unwrap();
    assert_eq!(py.contract.kind, alt_cli::verification::Kind::Tests);
    py.contract.validate(cwd.path(), &py.argv).unwrap();
}

#[test]
fn bounded_memory_keeps_current_evidence_and_whole_excerpts_after_restart() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    for n in 0..12 {
        fs::write(
            cwd.path().join(format!("cedar_{n}.py")),
            format!("def cedar_{n}(name):\n    return name.strip()\n"),
        )
        .unwrap();
    }
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        p.start_task("t", "Cedar repair").unwrap();
        p.pin("Cedar must preserve its public API").unwrap();
        p.note("t", "decision", "Cedar uses the bundled helper", "user")
            .unwrap();
        for n in 0..100 {
            p.note(
                "t",
                "decision",
                &format!("Unrelated panel spacing {n}"),
                "user",
            )
            .unwrap();
        }
    }
    let mut p = Project::open(data.path(), cwd.path()).unwrap();
    for budget in [1024, 4096, 8192] {
        let memory = p.memory("t", "Cedar", budget).unwrap();
        assert!(memory.chars().count() <= budget);
        assert!(memory.starts_with("CURRENT computed check status"));
        let view = p.context_view("t").unwrap();
        for excerpt in view["included_excerpts"].as_array().unwrap() {
            assert!(memory.contains(excerpt["excerpt"].as_str().unwrap()));
        }
        assert_eq!(
            memory.matches("[End excerpt]").count(),
            view["included_excerpts"].as_array().unwrap().len()
        );
    }
    fs::write(
        cwd.path().join("cedar_0.py"),
        "def changed_symbol(): return 1\n",
    )
    .unwrap();
    assert!(
        p.memory("t", "changed_symbol", 4096)
            .unwrap()
            .contains("changed_symbol")
    );
}

#[test]
fn retrieval_finds_named_files_and_important_terms_after_conversational_filler() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    fs::create_dir(cwd.path().join("nested")).unwrap();
    fs::write(
        cwd.path().join("nested/settings.py"),
        "TIMEOUT_SECONDS = 731\n",
    )
    .unwrap();
    fs::write(
        cwd.path().join("cache.py"),
        "def cedar_cache_ttl(): return 42\n",
    )
    .unwrap();
    let mut p = Project::open(data.path(), cwd.path()).unwrap();
    let named = p.search("Please inspect settings.py for me").unwrap();
    assert_eq!(named[0]["path"], "nested/settings.py");
    let long = p
        .search("I want you to help me with this please so we can do the cedar_cache_ttl repair")
        .unwrap();
    assert!(long.iter().any(|h| h["path"] == "cache.py"));
    p.start_task("unicode", "Inspect Unicode query").unwrap();
    let memory = p.memory("unicode", &"設定 ".repeat(500), 4096).unwrap();
    assert!(!memory.contains("Search needs 1–1,000 bytes"));
}
