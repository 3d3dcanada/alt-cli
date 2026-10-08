use std::fs;
#[test]
fn backup_restores_consistent_sqlite_and_never_overwrites_existing_state() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let db = rusqlite::Connection::open(root.path().join("sessions.db")).unwrap();
    db.execute_batch("PRAGMA journal_mode=WAL;CREATE TABLE evidence(value TEXT);INSERT INTO evidence VALUES('preserve');").unwrap();
    fs::write(
        root.path().join("preferences.toml"),
        "context_tokens = 8192\n",
    )
    .unwrap();
    fs::create_dir(root.path().join("blobs")).unwrap();
    fs::write(root.path().join("blobs/weight.gguf"), "exclude").unwrap();
    let archive = outside.path().join("backup.tar.gz");
    alt_cli::storage::backup(root.path(), &archive).unwrap();
    let dest = outside.path().join("restored");
    alt_cli::storage::restore(&archive, &dest).unwrap();
    let restored = rusqlite::Connection::open(dest.join("sessions.db")).unwrap();
    assert_eq!(
        restored
            .query_row("SELECT value FROM evidence", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "preserve"
    );
    assert!(!dest.join("blobs").exists());
    assert!(alt_cli::storage::restore(&archive, &dest).is_err());
    assert!(alt_cli::storage::backup(root.path(), &archive).is_err());
}
#[test]
fn malformed_archive_never_creates_destination() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("bad.gz");
    fs::write(&archive, b"invalid archive").unwrap();
    let dest = root.path().join("restored");
    assert!(alt_cli::storage::restore(&archive, &dest).is_err());
    assert!(!dest.exists());
}
#[test]
fn diagnostics_excludes_environment_values_prompts_endpoints_and_source() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("secret-log"), "super-secret-value").unwrap();
    let value = alt_cli::storage::diagnostics(root.path())
        .unwrap()
        .to_string();
    assert!(!value.contains("super-secret-value"));
    assert!(value.contains("omitted"));
}

#[test]
fn history_retention_archives_first_and_restores_without_overwriting() {
    use alt_cli::{
        config::{Profile, Provider},
        store::{Session, Store},
    };
    let root = tempfile::tempdir().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    for id in ["old-archived", "old-active", "recent-archived"] {
        store
            .create(&Session {
                id: id.into(),
                engine_id: "engine".into(),
                cwd: "/tmp/project".into(),
                profile_name: "local".into(),
                profile: Profile {
                    provider: Provider::Openai,
                    endpoint: "http://127.0.0.1:8080/v1".into(),
                    model: "fixture".into(),
                    context_tokens: 8192,
                    max_turns: 12,
                    uncensored: false,
                    api_key_env: None,
                    local_model: None,
                    inference: None,
                },
            })
            .unwrap();
        store
            .append(
                id,
                &serde_json::json!({"type":"user","text":"preserve this decision"}),
            )
            .unwrap();
        if id != "old-active" {
            store.archive(id, true).unwrap();
        }
    }
    {
        let db = rusqlite::Connection::open(root.path().join("sessions.db")).unwrap();
        db.execute("UPDATE session_details SET updated_at='2000-01-01 00:00:00' WHERE session_id LIKE 'old-%'",[]).unwrap();
    }
    let preview = store.retain_archived(root.path(), 30, false).unwrap();
    assert_eq!(preview["conversations"].as_array().unwrap().len(), 1);
    assert!(store.get("old-archived").is_ok());
    let report = store.retain_archived(root.path(), 30, true).unwrap();
    assert!(store.get("old-archived").is_err());
    assert!(store.get("old-active").is_ok());
    assert!(store.get("recent-archived").is_ok());
    let path = std::path::Path::new(report["recovery_archives"][0].as_str().unwrap());
    assert!(path.is_file());
    assert_eq!(store.restore_history(path).unwrap(), "old-archived");
    assert!(store.restore_history(path).is_err());
    let mut rows = Vec::new();
    store
        .visit_history("old-archived", |v| {
            rows.push(v.clone());
            Ok(())
        })
        .unwrap();
    assert_eq!(rows[0]["text"], "preserve this decision");
}

#[test]
fn retention_never_deletes_a_conversation_that_exceeds_restore_limits() {
    use alt_cli::{
        config::{Profile, Provider},
        store::{Session, Store},
    };
    let root = tempfile::tempdir().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    store
        .create(&Session {
            id: "too-large".into(),
            engine_id: "fixture".into(),
            cwd: "/tmp".into(),
            profile_name: "fixture".into(),
            profile: Profile {
                provider: Provider::Openai,
                endpoint: "http://127.0.0.1:1/v1".into(),
                model: "fixture".into(),
                context_tokens: 4096,
                max_turns: 1,
                uncensored: false,
                api_key_env: None,
                local_model: None,
                inference: None,
            },
        })
        .unwrap();
    store
        .append(
            "too-large",
            &serde_json::json!({"text":"x".repeat(16*1024*1024)}),
        )
        .unwrap();
    store.archive("too-large", true).unwrap();
    let db = rusqlite::Connection::open(root.path().join("sessions.db")).unwrap();
    db.execute(
        "UPDATE session_details SET updated_at='2000-01-01 00:00:00'",
        [],
    )
    .unwrap();
    assert!(
        store
            .retain_archived(root.path(), 30, true)
            .unwrap_err()
            .to_string()
            .contains("original retained")
    );
    assert!(store.get("too-large").is_ok());
    assert_eq!(
        db.query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn mutated_archive_corpus_preserves_atomic_restore_boundary() {
    let source = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(
        source.path().join("preferences.toml"),
        "context_tokens=4096\n",
    )
    .unwrap();
    let original = outside.path().join("original.tar.gz");
    alt_cli::storage::backup(source.path(), &original).unwrap();
    let bytes = fs::read(original).unwrap();
    let mut random = 0x414243444546u64;
    for n in 0..128 {
        let mut mutated = bytes.clone();
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        let index = random as usize % mutated.len();
        mutated[index] ^= 1 << (n % 8);
        if n % 4 == 0 {
            mutated.truncate(index);
        }
        let archive = outside.path().join("mutated.gz");
        fs::write(&archive, mutated).unwrap();
        let dest = outside.path().join(format!("restore-{n}"));
        match alt_cli::storage::restore(&archive, &dest) {
            Ok(_) => assert_eq!(
                fs::read(dest.join("preferences.toml")).unwrap(),
                b"context_tokens=4096\n"
            ),
            Err(_) => assert!(!dest.exists(), "Failed restore exposed partial state"),
        }
    }
}

#[test]
fn inference_retention_exports_inactive_payloads_but_protects_active_and_receipts() {
    use std::{
        fs::OpenOptions,
        time::{Duration, SystemTime},
    };
    let root = tempfile::tempdir().unwrap();
    for id in ["old", "active"] {
        let directory = root.path().join("inference").join(id);
        fs::create_dir_all(&directory).unwrap();
        for (name, data) in [
            ("effective.json", "{}"),
            ("call-request.json", "private input"),
            ("call-response.raw", "model output"),
            (
                "call-receipt.json",
                "{\"complete\":true,\"charged_generated_tokens\":12}",
            ),
            ("active.lock", ""),
        ] {
            let path = directory.join(name);
            fs::write(&path, data).unwrap();
            let old = SystemTime::now() - Duration::from_secs(86400 * 10);
            fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_times(fs::FileTimes::new().set_modified(old))
                .unwrap();
        }
    }
    let active = OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.path().join("inference/active/active.lock"))
        .unwrap();
    fs2::FileExt::lock_shared(&active).unwrap();
    let preview = alt_cli::storage::retain(root.path(), 1, false).unwrap();
    assert_eq!(
        preview["inference"]["connections"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(root.path().join("inference/old/call-request.json").exists());
    let result = alt_cli::storage::retain(root.path(), 1, true).unwrap();
    assert!(!root.path().join("inference/old/call-request.json").exists());
    assert!(!root.path().join("inference/old/call-response.raw").exists());
    assert!(root.path().join("inference/old/call-receipt.json").exists());
    assert!(
        root.path()
            .join("inference/active/call-request.json")
            .exists()
    );
    let export = &result["inference"]["connections"][0]["export"];
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(
        fs::File::open(export["archive"].as_str().unwrap()).unwrap(),
    ));
    let names: Vec<_> = archive
        .entries()
        .unwrap()
        .map(|entry| entry.unwrap().path().unwrap().to_string_lossy().to_string())
        .collect();
    assert!(names.contains(&"inference/call-request.json".to_string()));
    assert!(names.contains(&"INFERENCE-MANIFEST.json".to_string()));
    assert!(
        alt_cli::storage::export_inference(
            root.path(),
            "active",
            &root.path().join("active.tar.gz")
        )
        .is_err()
    );
}

#[test]
fn large_archival_payload_cannot_block_core_backup_and_receipts_survive_restore() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let directory = root.path().join("inference/old");
    fs::create_dir_all(&directory).unwrap();
    let payload = fs::File::create(directory.join("call-response.raw")).unwrap();
    payload.set_len(600 * 1024 * 1024).unwrap();
    fs::write(directory.join("call-receipt.json"), "{\"complete\":true}").unwrap();
    fs::write(
        root.path().join("preferences.toml"),
        "context_tokens=8192\n",
    )
    .unwrap();
    let archive = outside.path().join("core.tar.gz");
    let receipt = alt_cli::storage::backup(root.path(), &archive).unwrap();
    assert!(receipt["generation"].is_string());
    assert!(archive.metadata().unwrap().len() < 1024 * 1024);
    let restored = outside.path().join("restored");
    alt_cli::storage::restore(&archive, &restored).unwrap();
    assert_eq!(
        fs::read(restored.join("inference/old/call-receipt.json")).unwrap(),
        b"{\"complete\":true}"
    );
    assert!(!restored.join("inference/old/call-response.raw").exists());
    assert!(directory.join("call-response.raw").exists());
}

#[test]
fn snapshot_refuses_mutating_generation_and_active_execution_without_publishing() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let destination = outside.path().join("backup.tar.gz");
    let guard = alt_cli::storage::StateWriteGuard::acquire(root.path()).unwrap();
    assert!(
        alt_cli::storage::backup(root.path(), &destination)
            .unwrap_err()
            .to_string()
            .contains("State is changing")
    );
    assert!(!destination.exists());
    drop(guard);
    let execution = root.path().join("projects/project/execution/run");
    fs::create_dir_all(&execution).unwrap();
    let lease = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(execution.join("active.lock"))
        .unwrap();
    fs2::FileExt::lock_shared(&lease).unwrap();
    assert!(
        alt_cli::storage::backup(root.path(), &destination)
            .unwrap_err()
            .to_string()
            .contains("execution receipt")
    );
    assert!(!destination.exists());
    drop(lease);
    alt_cli::storage::backup(root.path(), &destination).unwrap();
}

#[test]
fn state_writers_return_bounded_busy_without_changing_settings() {
    use std::time::{Duration, Instant};
    let root = tempfile::tempdir().unwrap();
    alt_cli::config::Preferences::default()
        .save(root.path())
        .unwrap();
    let original = fs::read(root.path().join("preferences.toml")).unwrap();
    let barrier = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.path().join("state-generation.lock"))
        .unwrap();
    fs2::FileExt::lock_exclusive(&barrier).unwrap();
    let started = Instant::now();
    let error = alt_cli::config::Preferences::default()
        .save(root.path())
        .unwrap_err();
    assert!(error.to_string().contains("backup is freezing state"));
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(
        fs::read(root.path().join("preferences.toml")).unwrap(),
        original
    );
    drop(barrier);
    alt_cli::config::Preferences::default()
        .save(root.path())
        .unwrap();
}
