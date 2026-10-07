use alt_cli::{
    config::{Config, Preferences, Profile, Provider},
    practice,
    project::{Policy, Project},
    store::{Session, Store},
    workflow,
};
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs,
    sync::{Arc, atomic::AtomicBool},
};

fn profile() -> Profile {
    Profile {
        provider: Provider::Openai,
        endpoint: "http://127.0.0.1:1/v1".into(),
        model: "explicit-fixture".into(),
        context_tokens: 8192,
        max_turns: 12,
        uncensored: true,
        api_key_env: None,
        local_model: None,
        inference: None,
    }
}

#[test]
fn isolated_pc_state_preserves_selection_without_copying_or_touching_user_work() {
    let source = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let original = profile();
    Config {
        default_profile: "chosen".into(),
        profiles: BTreeMap::from([
            ("chosen".into(), original.clone()),
            ("another".into(), profile()),
        ]),
    }
    .save(source.path())
    .unwrap();
    let prefs = Preferences {
        project: project.path().into(),
        runtime_path: None,
        ..Preferences::default()
    };
    prefs.save(source.path()).unwrap();
    fs::write(project.path().join("keep.py"), "unchanged").unwrap();
    let settings = fs::read(source.path().join("config.toml")).unwrap();
    let preferences = fs::read(source.path().join("preferences.toml")).unwrap();
    let engine = std::env::current_exe().unwrap();
    let isolated = destination.path().join("new-state");
    let test = practice::prepare_test(source.path(), &isolated, Some("chosen"), &engine).unwrap();
    assert_ne!(test.lesson.project, project.path());
    assert_eq!(
        fs::read(project.path().join("keep.py")).unwrap(),
        b"unchanged"
    );
    assert_eq!(
        fs::read(source.path().join("config.toml")).unwrap(),
        settings
    );
    assert_eq!(
        fs::read(source.path().join("preferences.toml")).unwrap(),
        preferences
    );
    let copied = Config::read(&isolated).unwrap();
    assert_eq!(copied.profiles.len(), 1);
    assert_eq!(
        serde_json::to_value(copied.profile(None).unwrap().1).unwrap(),
        serde_json::to_value(
            Config::read(source.path())
                .unwrap()
                .profile(Some("chosen"))
                .unwrap()
                .1
        )
        .unwrap()
    );
    assert!(practice::prepare_test(source.path(), &isolated, Some("chosen"), &engine).is_err());
    let mut denied = Config::read(source.path()).unwrap();
    denied.profiles.get_mut("chosen").unwrap().uncensored = false;
    denied.save(source.path()).unwrap();
    assert!(
        practice::prepare_test(
            source.path(),
            &destination.path().join("denied"),
            Some("chosen"),
            &engine
        )
        .is_err()
    );
    assert!(!destination.path().join("denied").exists());
}

#[test]
fn reviewed_allocation_survives_restart_and_cannot_replace_the_saved_model() {
    let root = tempfile::tempdir().unwrap();
    let original = profile();
    let session = Session {
        id: "saved".into(),
        engine_id: "engine".into(),
        cwd: root.path().display().to_string(),
        profile_name: "chosen".into(),
        profile: original.clone(),
    };
    let store = Store::open(root.path()).unwrap();
    store.create(&session).unwrap();
    store
        .append("saved", &json!({"type":"user","text":"Original task ☃"}))
        .unwrap();
    let mut changed = original.clone();
    changed.context_tokens = 16384;
    changed.inference = Some(alt_cli::inference::Settings::legacy(16384));
    store.update_allocation("saved", changed.clone()).unwrap();
    drop(store);
    let store = Store::open(root.path()).unwrap();
    assert_eq!(
        serde_json::to_value(store.get("saved").unwrap().profile).unwrap(),
        serde_json::to_value(&changed).unwrap()
    );
    let history = store.history("saved").unwrap();
    assert!(
        history
            .iter()
            .any(|e| e["type"] == "allocation_changed" && e["model_unchanged"] == true)
    );
    assert!(history.iter().any(|e| e["text"] == "Original task ☃"));
    changed.model = "silent-fallback".into();
    assert!(store.update_allocation("saved", changed).is_err());
    assert_eq!(store.get("saved").unwrap().profile.model, original.model);
}

#[tokio::test]
async fn actual_failed_cases_unchanged_checks_source_cycles_and_report_tampering() {
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
    assert_eq!(run().await.unwrap().exit_code, Some(1));
    assert_eq!(run().await.unwrap().exit_code, Some(1));
    {
        let p = Project::open(data.path(), &lesson.project).unwrap();
        let r = workflow::recovery(&p, &lesson.task).unwrap().unwrap();
        assert_eq!(r["unchanged_revision_failures"], 2);
        assert_eq!(r["failed_case_count"], 3);
        assert!(
            r["failed_cases"]
                .as_array()
                .unwrap()
                .contains(&json!("trim spaces"))
        );
        assert!(
            p.task(&lesson.task)
                .unwrap()
                .next
                .contains("unchanged source")
        );
    }
    let replace = |body: &str| {
        let p = Project::open(data.path(), &lesson.project).unwrap();
        let read = p.read(&lesson.task, "greeting.py", 1, 100).unwrap();
        let change = p
            .prepare_text_edit(
                &lesson.task,
                "greeting.py",
                read["range_handle"].as_str().unwrap(),
                body,
                "Testing recovery from a source cycle",
            )
            .unwrap();
        p.apply(&change.id, Policy::Trusted).unwrap();
    };
    replace("def greet(name):\n    return 'bad'\n");
    assert_eq!(run().await.unwrap().exit_code, Some(1));
    replace(practice::SEED);
    let repeated = run().await.unwrap();
    let p = Project::open(data.path(), &lesson.project).unwrap();
    let r = workflow::recovery(&p, &lesson.task).unwrap().unwrap();
    assert_eq!(r["revisited_failed_revision"], true);
    assert_eq!(r["same_observed_failure_count"], 3);
    fs::write(
        p.state
            .join("check-reports")
            .join(format!("{}.report", repeated.id)),
        "tampered",
    )
    .unwrap();
    assert_eq!(
        workflow::recovery(&p, &lesson.task).unwrap().unwrap()["failed_cases"],
        json!([])
    );
    drop(p);
    replace("def greet(name):\n    return 'Hello, ' + (name.strip() or 'friend') + '!'\n");
    assert_eq!(run().await.unwrap().exit_code, Some(0));
    assert!(
        workflow::recovery(
            &Project::open(data.path(), &lesson.project).unwrap(),
            &lesson.task
        )
        .unwrap()
        .is_none()
    );
}
