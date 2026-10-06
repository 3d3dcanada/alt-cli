use alt_cli::{
    config::Preferences,
    config::{Profile, Provider},
    storage,
    store::{Session, Store},
};
use std::{fs, process::Command};
#[test]
fn recovery_worker() {
    let Ok(root) = std::env::var("ALT_RECOVERY_ROOT") else {
        return;
    };
    let root = std::path::Path::new(&root);
    let result = match std::env::var("ALT_RECOVERY_OPERATION").unwrap().as_str() {
        "settings" => {
            let mut p = Preferences::load(root).unwrap();
            p.context_tokens = 4096;
            p.runtime.threads = 3;
            p.save(root).map(|_| ())
        }
        "backup" => storage::backup(
            std::path::Path::new(&std::env::var("ALT_RECOVERY_SOURCE").unwrap()),
            &root.join("backup.tar.gz"),
        )
        .map(|_| ()),
        "restore" => storage::restore(
            std::path::Path::new(&std::env::var("ALT_RECOVERY_ARCHIVE").unwrap()),
            &root.join("restored"),
        )
        .map(|_| ()),
        _ => panic!("operation"),
    };
    if std::env::var("ALT_FAULT_MODE").unwrap() == "enospc" {
        assert!(result.is_err());
    } else {
        panic!("fault did not activate: {result:?}");
    }
}
#[test]
fn settings_backup_restore_survive_disk_full_and_abrupt_atomic_writes() {
    let tool = tempfile::tempdir().unwrap();
    let library = tool.path().join("fault.so");
    assert!(
        Command::new("cc")
            .args(["-shared", "-fPIC", "-O2"])
            .arg(format!(
                "{}/tests/fixtures/fault_io.c",
                env!("CARGO_MANIFEST_DIR")
            ))
            .args(["-ldl", "-o"])
            .arg(&library)
            .status()
            .unwrap()
            .success()
    );
    for operation in ["settings", "backup", "restore"] {
        for mode in ["enospc", "crash-before-rename"] {
            let source = tempfile::tempdir().unwrap();
            Preferences::default().save(source.path()).unwrap();
            Store::open(source.path()).unwrap();
            let archive = tool.path().join(format!("{operation}-{mode}.tar.gz"));
            storage::backup(source.path(), &archive).unwrap();
            let target = tempfile::tempdir().unwrap();
            Preferences::default().save(target.path()).unwrap();
            let original = fs::read(target.path().join("preferences.toml")).unwrap();
            let result = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "recovery_worker", "--nocapture"])
                .env("LD_PRELOAD", &library)
                .env("ALT_FAULT_MODE", mode)
                .env("ALT_FAULT_MATCH", "*")
                .env("ALT_FAULT_PROJECT", target.path())
                .env("ALT_RECOVERY_ROOT", target.path())
                .env("ALT_RECOVERY_OPERATION", operation)
                .env("ALT_RECOVERY_SOURCE", source.path())
                .env("ALT_RECOVERY_ARCHIVE", &archive)
                .output()
                .unwrap();
            assert_eq!(
                result.status.code(),
                Some(if mode == "enospc" { 0 } else { 86 }),
                "{operation}/{mode}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(
                fs::read(target.path().join("preferences.toml")).unwrap(),
                original
            );
            assert!(!target.path().join("backup.tar.gz").exists());
            assert!(!target.path().join("restored").exists());
            let restored = target.path().join("retry");
            storage::restore(&archive, &restored).unwrap();
            Preferences::load(&restored).unwrap();
        }
    }
}
#[test]
fn session_metadata_and_events_commit_together_and_inline_preferences_ignore_legacy_file() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path()).unwrap();
    let session = Session {
        id: "t".into(),
        engine_id: "e".into(),
        cwd: "/tmp".into(),
        profile_name: "fixture".into(),
        profile: Profile {
            provider: Provider::Openai,
            endpoint: "http://127.0.0.1:1/v1".into(),
            model: "fixture".into(),
            context_tokens: 2048,
            max_turns: 1,
            uncensored: false,
            api_key_env: None,
            local_model: None,
        },
    };
    store.create(&session).unwrap();
    let db = rusqlite::Connection::open(root.path().join("sessions.db")).unwrap();
    db.execute_batch("CREATE TRIGGER fail_details BEFORE UPDATE ON session_details BEGIN SELECT RAISE(ABORT,'injected metadata failure'); END;").unwrap();
    assert!(
        store
            .append(
                "t",
                &serde_json::json!({"type":"user","text":"must rollback"})
            )
            .is_err()
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    let mut prefs = Preferences {
        context_tokens: 4096,
        ..Default::default()
    };
    prefs.runtime.threads = 3;
    prefs.save(root.path()).unwrap();
    fs::write(root.path().join("runtime.toml"), "broken legacy runtime").unwrap();
    let loaded = Preferences::load(root.path()).unwrap();
    assert_eq!(loaded.context_tokens, 4096);
    assert_eq!(loaded.runtime.threads, 3);
}
