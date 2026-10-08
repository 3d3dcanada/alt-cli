//! Opt-in persistence workload. These are synthetic history events, not model results.
use alt_cli::{
    config::{Profile, Provider},
    project::Project,
    storage,
    store::{Session, Store},
};
use serde_json::json;
use std::{path::PathBuf, sync::atomic::AtomicBool, time::Instant};

#[test]
#[ignore = "persistent 2,000-session qualification; run explicitly with --ignored --nocapture"]
fn two_thousand_sessions_survive_retention_and_restoration() {
    let temporary = tempfile::tempdir().unwrap();
    let root = std::env::var_os("ALT_LONGEVITY_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temporary.path().join("run"));
    assert!(
        !root.exists(),
        "Use a new longevity root; keep prior evidence"
    );
    std::fs::create_dir_all(&root).unwrap();
    let state = root.join("state");
    let started = Instant::now();
    let profile = Profile {
        provider: Provider::Openai,
        endpoint: "http://127.0.0.1:1/v1".into(),
        model: "deterministic-persistence-fixture-no-weights".into(),
        context_tokens: 8192,
        max_turns: 12,
        uncensored: false,
        api_key_env: None,
        local_model: None,
        inference: None,
    };
    let projects: Vec<_> = (0..5)
        .map(|n| {
            let path = root.join(format!("project-{n}"));
            std::fs::create_dir(&path).unwrap();
            std::fs::write(path.join("source.py"), "VALUE = 0\n").unwrap();
            path
        })
        .collect();
    let mut store = Store::open(&state).unwrap();
    for n in 0..2000 {
        let id = format!("longevity-{n:04}");
        store
            .create(&Session {
                id: id.clone(),
                engine_id: id.clone(),
                cwd: projects[n % 5].to_string_lossy().into_owned(),
                profile_name: "fixture".into(),
                profile: profile.clone(),
            })
            .unwrap();
        for event in [
            json!({"type":"user","text":format!("Request {n}: preserve café and requirement {n}")}),
            json!({"type":"fixture_observation","text":"No model was invoked"}),
            json!({"type":"cancelled","fixture":true}),
            json!({"type":"user","text":format!("Recovery request {n}")}),
        ] {
            store.append(&id, &event).unwrap();
        }
        if n < 1000 {
            store.archive(&id, true).unwrap();
        }
        if n % 40 == 0 {
            let project = &projects[(n / 40) % 5];
            let mut p = Project::open(&state, project).unwrap();
            p.start_task(&id, &format!("Observed external edit {n}"))
                .unwrap();
            p.index_incremental(None).unwrap();
            drop(p);
            std::fs::write(project.join("source.py"), format!("VALUE = {}\n", n + 1)).unwrap();
            let mut p = Project::open(&state, project).unwrap();
            assert_eq!(p.index_incremental(None).unwrap().updated, 1);
            assert!(p.index_incremental(Some(&AtomicBool::new(true))).is_err());
            p.index_incremental(None).unwrap();
        }
    }
    assert_eq!(store.list().unwrap().len(), 2000);
    for n in [0, 999, 1000, 1999] {
        let history = store.history(&format!("longevity-{n:04}")).unwrap();
        assert_eq!(history.len(), 4);
        assert!(
            history[0]["text"]
                .as_str()
                .unwrap()
                .contains(&format!("requirement {n}"))
        );
    }
    // Advance only the fixture's archival clock, avoiding wall-clock sleeps.
    rusqlite::Connection::open(state.join("sessions.db"))
        .unwrap()
        .execute(
            "UPDATE session_details SET updated_at='2000-01-01 00:00:00' WHERE archived=1",
            [],
        )
        .unwrap();
    let before = storage::usage(&state).unwrap();
    let retention = store.retain_archived(&state, 30, true).unwrap();
    assert_eq!(
        retention["recovery_archives"].as_array().unwrap().len(),
        1000
    );
    assert_eq!(store.list().unwrap().len(), 1000);
    let archive = PathBuf::from(retention["recovery_archives"][0].as_str().unwrap());
    let recovered = store.restore_history(&archive).unwrap();
    assert_eq!(store.history(&recovered).unwrap().len(), 4);
    // Exercise aged request payloads as well as conversation retention. These are
    // explicitly synthetic receipts, never live inference or model measurements.
    let old = std::time::SystemTime::now() - std::time::Duration::from_secs(86400 * 60);
    for n in 0..33 {
        let directory = state.join("inference").join(format!("fixture-{n:03}"));
        std::fs::create_dir_all(&directory).unwrap();
        for (name, data) in [
            ("active.lock", ""),
            ("effective.json", "{\"fixture\":true}"),
            ("call-request.json", "{\"fixture\":true,\"messages\":[]}"),
            ("call-response.raw", "synthetic response, no weights"),
            (
                "call-receipt.json",
                "{\"fixture\":true,\"complete\":true,\"charged_generated_tokens\":0}",
            ),
        ] {
            let path = directory.join(name);
            std::fs::write(&path, data).unwrap();
            std::fs::File::options()
                .write(true)
                .open(path)
                .unwrap()
                .set_times(std::fs::FileTimes::new().set_modified(old))
                .unwrap();
        }
    }
    let active = std::fs::File::options()
        .read(true)
        .write(true)
        .open(state.join("inference/fixture-032/active.lock"))
        .unwrap();
    fs2::FileExt::lock_shared(&active).unwrap();
    let inference_retention = storage::retain(&state, 30, true).unwrap();
    assert_eq!(
        inference_retention["inference"]["connections"]
            .as_array()
            .unwrap()
            .len(),
        32
    );
    assert_eq!(
        inference_retention["inference"]["protected"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    for n in 0..32 {
        let directory = state.join("inference").join(format!("fixture-{n:03}"));
        assert!(!directory.join("call-request.json").exists());
        assert!(!directory.join("call-response.raw").exists());
        assert!(directory.join("call-receipt.json").exists());
        assert!(directory.join("archived.json").exists());
    }
    assert!(
        state
            .join("inference/fixture-032/call-request.json")
            .exists()
    );
    drop(active);
    let before_backup = store.list().unwrap();
    drop(store);
    let backup = root.join("longevity-state.tar.gz");
    let backup_receipt = storage::backup(&state, &backup).unwrap();
    let restored = root.join("restored");
    let restore_receipt = storage::restore(&backup, &restored).unwrap();
    let reopened = Store::open(&restored).unwrap();
    assert_eq!(reopened.list().unwrap(), before_backup);
    assert_eq!(reopened.history("longevity-1999").unwrap().len(), 4);
    assert_eq!(reopened.history(&recovered).unwrap().len(), 4);
    for n in 0..33 {
        let directory = restored.join("inference").join(format!("fixture-{n:03}"));
        assert!(directory.join("call-receipt.json").exists());
        assert!(!directory.join("call-response.raw").exists());
    }
    assert_eq!(
        rusqlite::Connection::open(restored.join("sessions.db"))
            .unwrap()
            .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    for project in &projects {
        let p = Project::open(&restored, project).unwrap();
        assert!(p.snapshot().is_ok());
    }
    let receipt = json!({
        "schema":1,"passed":true,"sessions_created":2000,"events_written":8000,
        "projects":5,"external_edit_index_refreshes":50,"cancelled_indexes":50,
        "archived_sessions":1000,"restored_history_sessions":1,
        "synthetic_inference_connections":33,"inference_payloads_exported":32,"active_inference_connections_protected":1,
        "sessions_after_retention_and_restore":1001,"elapsed_seconds":started.elapsed().as_secs_f64(),
        "state":state,"restored_state":restored,"storage_before":before,
        "inference_retention":inference_retention,"backup":backup_receipt,"restore":restore_receipt,
        "scope":"Actual Store/Project persistence operations with synthetic events. No model/GPU or novice qualification. Installer update must consume this persistent state in a separate package gate.",
        "installer_update_performed":false,"model_requests":0
    });
    std::fs::write(
        root.join("longevity-receipt.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string(&receipt).unwrap());
}
