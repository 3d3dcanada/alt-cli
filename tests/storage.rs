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
