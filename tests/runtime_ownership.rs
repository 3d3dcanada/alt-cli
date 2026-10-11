//! Owned llama-server launch: per-launch key, private key file, authenticated readiness.
use alt_cli::{
    config::{Preferences, Profile, Provider},
    models, runtime,
};
use std::sync::{Arc, atomic::AtomicBool};

async fn fixture_model(root: &std::path::Path) -> String {
    let original = root.join("fixture.gguf");
    std::fs::write(&original, fixture_gguf()).unwrap();
    models::import(
        root,
        &original,
        true,
        Arc::new(AtomicBool::new(false)),
        |_| {},
    )
    .await
    .unwrap()
    .id
}

/// Smallest structurally valid GGUF v3 header: no tensors, no metadata.
pub fn fixture_gguf() -> Vec<u8> {
    let mut bytes = b"GGUF".to_vec();
    bytes.extend(3u32.to_le_bytes());
    bytes.extend(0u64.to_le_bytes());
    bytes.extend(0u64.to_le_bytes());
    bytes
}

fn profile(model: String) -> Profile {
    Profile {
        provider: Provider::Openai,
        endpoint: "http://127.0.0.1:1/v1".into(),
        model: "fixture-alias".into(),
        context_tokens: 4096,
        max_turns: 2,
        uncensored: true,
        api_key_env: None,
        local_model: Some(model),
        inference: None,
        runtime_key: None,
    }
}

#[tokio::test]
async fn owned_runtime_requires_its_per_launch_key_and_keeps_it_out_of_argv() {
    let root = tempfile::tempdir().unwrap();
    let model = fixture_model(root.path()).await;
    let argv_file = root.path().join("argv.json");
    // SAFETY: this test binary runs this single test that reads the variable in a child.
    unsafe { std::env::set_var("ALT_FAKE_ARGV", &argv_file) };
    let preferences = Preferences {
        runtime_path: Some(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/fake_llama_server.py"),
        ),
        ..Preferences::default()
    };
    let mut selected = profile(model);
    let mut owned = runtime::LocalRuntime::start(
        root.path(),
        &preferences,
        &selected,
        Arc::new(AtomicBool::new(false)),
        |_| {},
    )
    .await
    .unwrap();
    let argv: Vec<String> = serde_json::from_slice(&std::fs::read(&argv_file).unwrap()).unwrap();
    assert!(argv.iter().any(|a| a == "--api-key-file"));
    assert!(!argv.iter().any(|a| a == "--api-key"), "{argv:?}");
    assert!(
        !argv.iter().any(|a| a.contains(&owned.api_key)),
        "key leaked into argv"
    );
    assert!(owned.api_key.len() >= 32);
    // The key file is removed once the runtime is ready.
    let key_file = &argv[argv.iter().position(|a| a == "--api-key-file").unwrap() + 1];
    assert!(!std::path::Path::new(key_file).exists());
    let client = reqwest::Client::new();
    let anonymous = client
        .get(format!("{}/models", owned.endpoint))
        .send()
        .await
        .unwrap();
    assert_eq!(anonymous.status(), 401);
    owned.attach(&mut selected);
    assert_eq!(
        selected.key().unwrap().as_deref(),
        Some(owned.api_key.as_str())
    );
    let serialized = toml::to_string(&selected).unwrap();
    assert!(
        !serialized.contains(&owned.api_key),
        "key must never be saved"
    );
    let authorized = client
        .get(format!("{}/models", owned.endpoint))
        .bearer_auth(&owned.api_key)
        .send()
        .await
        .unwrap();
    assert!(authorized.status().is_success());
    owned.stop().await;
}
