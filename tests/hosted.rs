//! A03: provider presets and hosted model discovery, headless.
#[path = "fixtures/upstream.rs"]
mod upstream;

use serde_json::json;

fn alt(root: &std::path::Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_alt"))
        .arg("--data-dir")
        .arg(root)
        .args(args)
        .env_remove("NVIDIA_API_KEY")
        .output()
        .unwrap()
}

#[test]
fn init_with_the_nvidia_preset_stores_only_the_variable_name() {
    let root = tempfile::tempdir().unwrap();
    let out = alt(
        root.path(),
        &["init", "--preset", "nvidia", "--model", "z-ai/glm-5.3"],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("Set NVIDIA_API_KEY"));
    let config: toml::Value =
        toml::from_str(&std::fs::read_to_string(root.path().join("config.toml")).unwrap()).unwrap();
    let profile = &config["profiles"]["local"];
    assert_eq!(
        profile["endpoint"].as_str(),
        Some("https://integrate.api.nvidia.com/v1")
    );
    assert_eq!(profile["api_key_env"].as_str(), Some("NVIDIA_API_KEY"));
    assert_eq!(profile["model"].as_str(), Some("z-ai/glm-5.3"));
    assert_eq!(
        profile["inference"]["dialect"].as_str(),
        Some("chat-template")
    );
    let unknown = alt(
        root.path(),
        &["init", "--preset", "nowhere", "--model", "m"],
    );
    assert!(!unknown.status.success());
}

#[tokio::test]
async fn hosted_catalogs_are_listed_and_filtered() {
    let catalog = upstream::Reply::json(json!({"object":"list","data":[
        {"id":"z-ai/glm-5.3"},{"id":"moonshotai/kimi-k3"},{"id":"qwen/qwen3-coder-480b"},{"id":"z-ai/glm-4.6"}
    ]}));
    let server = upstream::start(vec![catalog]).await;
    let root = tempfile::tempdir().unwrap();
    let endpoint = server.endpoint.clone();
    let root_path = root.path().to_owned();
    let listing = tokio::task::spawn_blocking(move || {
        let init = alt(
            &root_path,
            &[
                "init",
                "--preset",
                "openrouter",
                "--endpoint",
                &endpoint,
                "--model",
                "z-ai/glm-5.3",
                "--api-key-env",
                "ALT_UNUSED_FIXTURE_KEY",
            ],
        );
        assert!(
            init.status.success(),
            "{}",
            String::from_utf8_lossy(&init.stderr)
        );
        std::process::Command::new(env!("CARGO_BIN_EXE_alt"))
            .arg("--data-dir")
            .arg(&root_path)
            .args(["models", "--filter", "glm"])
            .env("ALT_UNUSED_FIXTURE_KEY", "fixture-placeholder")
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    assert!(
        listing.status.success(),
        "{}",
        String::from_utf8_lossy(&listing.stderr)
    );
    let text = String::from_utf8_lossy(&listing.stdout);
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        ["z-ai/glm-4.6", "z-ai/glm-5.3"]
    );
    let requests = server.requests.lock().unwrap().clone();
    assert!(requests[0].0.starts_with("GET /v1/models"));
    assert!(requests[0].0.contains("Bearer fixture-placeholder"));
}
