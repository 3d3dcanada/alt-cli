//! A02: open request parameters, Ollama context and the owned runtime argv.
#[path = "fixtures/upstream.rs"]
mod upstream;

use alt_cli::{
    config::{Profile, Provider},
    inference::{Dialect, OllamaContext, Relay, Settings},
    runtime,
};
use serde_json::json;
use std::path::Path;
use upstream::Reply;

fn profile(endpoint: &str, provider: Provider, settings: Settings) -> Profile {
    Profile {
        provider,
        endpoint: endpoint.into(),
        model: "fixture".into(),
        context_tokens: 16384,
        max_turns: 2,
        uncensored: true,
        api_key_env: None,
        local_model: None,
        inference: Some(settings),
        runtime_key: None,
    }
}

fn apply(p: &Profile) -> anyhow::Result<serde_json::Value> {
    let mut body = json!({"model":"fixture","messages":[],"stream":false,"temperature":0.2});
    p.effective_inference().apply(p, &mut body)?;
    Ok(body)
}

#[test]
fn extra_body_merges_last_and_alt_owned_keys_are_refused() {
    let mut s = Settings::legacy(16384);
    s.temperature = Some(0.4);
    s.presence_penalty = Some(0.5);
    s.seed = Some(7);
    s.stop = vec!["</done>".into()];
    s.extra_body = Some(r#"{"temperature":0.9,"top_a":0.1,"presence_penalty":0}"#.into());
    let p = profile("http://127.0.0.1:9/v1", Provider::Openai, s.clone());
    let body = apply(&p).unwrap();
    assert_eq!(
        body["temperature"], 0.9,
        "extra body wins over named settings"
    );
    assert_eq!(body["presence_penalty"], 0);
    assert_eq!(body["top_a"], 0.1);
    assert_eq!(body["seed"], 7);
    assert_eq!(body["stop"], json!(["</done>"]));
    for owned in [
        "model",
        "messages",
        "tools",
        "stream",
        "max_tokens",
        "max_completion_tokens",
    ] {
        let mut bad = s.clone();
        bad.extra_body = Some(format!(r#"{{"{owned}":1}}"#));
        let error = profile("http://127.0.0.1:9/v1", Provider::Openai, bad)
            .validate()
            .unwrap_err()
            .to_string();
        assert!(error.contains(owned), "{error}");
    }
    let mut not_object = s.clone();
    not_object.extra_body = Some("[1,2]".into());
    assert!(
        profile("http://127.0.0.1:9/v1", Provider::Openai, not_object)
            .validate()
            .is_err()
    );
}

#[test]
fn penalties_use_each_servers_spelling_or_are_refused() {
    let mut s = Settings::legacy(16384);
    s.repeat_penalty = Some(1.1);
    let generic = profile("http://127.0.0.1:9/v1", Provider::Openai, s.clone());
    assert!(
        generic.validate().is_err(),
        "OpenAI-style APIs have no repeat penalty"
    );
    s.dialect = Dialect::ChatTemplate;
    let vllm = profile("http://127.0.0.1:9/v1", Provider::Openai, s.clone());
    assert_eq!(apply(&vllm).unwrap()["repetition_penalty"], 1.1);
    s.dialect = Dialect::LlamaCpp;
    s.typical_p = Some(0.9);
    let llama = profile("http://127.0.0.1:9/v1", Provider::Openai, s);
    let body = apply(&llama).unwrap();
    assert_eq!(body["repeat_penalty"], 1.1);
    assert_eq!(body["typical_p"], 0.9);
}

#[tokio::test]
async fn ollama_receives_num_ctx_and_alt_reads_back_the_loaded_context() {
    let chat = Reply::json(
        json!({"choices":[{"message":{"role":"assistant","content":"ok"},"finish_reason":"stop"}],"usage":{"completion_tokens":2}}),
    );
    let ps = Reply::json(
        json!({"models":[{"name":"fixture","model":"fixture","context_length":16384}]}),
    );
    let server = upstream::start(vec![chat, ps]).await;
    let root = tempfile::tempdir().unwrap();
    let mut s = Settings::legacy(16384);
    s.keep_alive = Some("30m".into());
    let endpoint = server.endpoint.trim_end_matches("/v1").to_owned();
    let p = profile(&endpoint, Provider::Ollama, s.clone());
    let relay = Relay::start(root.path(), &p).await.unwrap();
    reqwest::Client::new()
        .post(format!("{}/chat/completions", relay.endpoint))
        .json(&json!({"model":"fixture","messages":[{"role":"user","content":"hi"}],"stream":false,"options":{"num_predict":5}}))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let requests = server.requests.lock().unwrap().clone();
    assert!(requests[0].0.starts_with("POST /v1/chat/completions"));
    assert_eq!(
        requests[0].1["options"],
        json!({"num_ctx":16384}),
        "engine options replaced"
    );
    assert_eq!(requests[0].1["keep_alive"], "30m");
    assert!(requests[1].0.starts_with("GET /api/ps"));
    assert_eq!(relay.status().ollama_context, Some(16384));
    // "Server default" sends no context at all.
    s.ollama_context = OllamaContext::ServerDefault;
    let quiet = profile(&endpoint, Provider::Ollama, s);
    assert!(apply(&quiet).unwrap().get("options").is_none());
}

#[test]
fn owned_runtime_argv_is_exact_and_refuses_alt_owned_flags() {
    let s = runtime::Settings {
        gpu_layers: 99,
        threads: 6,
        cache_k: "q8_0".into(),
        cache_v: "q8_0".into(),
        flash_attention: true,
        n_cpu_moe: Some(12),
        tensor_split: Some("3,1".into()),
        main_gpu: Some(0),
        rope_scaling: Some("yarn".into()),
        rope_scale: Some(2.0),
        yarn_orig_ctx: Some(32768),
        mlock: true,
        draft_model: Some("/models/draft.gguf".into()),
        draft_max: Some(8),
        extra_args: vec!["--top-nsigma".into(), "1.5".into()],
        ..Default::default()
    };
    s.validate().unwrap();
    let p = profile(
        "http://127.0.0.1:9/v1",
        Provider::Openai,
        Settings::legacy(16384),
    );
    let argv: Vec<String> = runtime::server_args(
        &s,
        &p,
        Path::new("/models/m.gguf"),
        4242,
        6,
        Path::new("/run/key"),
    )
    .into_iter()
    .map(|a| a.to_string_lossy().into_owned())
    .collect();
    let expected = "--model /models/m.gguf --ctx-size 16384 --host 127.0.0.1 --port 4242 --threads 6 --parallel 1 --n-gpu-layers 99 --batch-size 128 --ubatch-size 128 --cache-type-k q8_0 --cache-type-v q8_0 --flash-attn on --jinja --alias fixture --api-key-file /run/key --n-cpu-moe 12 --tensor-split 3,1 --main-gpu 0 --rope-scaling yarn --rope-scale 2 --yarn-orig-ctx 32768 --model-draft /models/draft.gguf --draft-max 8 --mlock --top-nsigma 1.5";
    assert_eq!(argv.join(" "), expected);
    for owned in [
        "--host",
        "--port=9",
        "--api-key",
        "--alias",
        "-m",
        "--ctx-size",
    ] {
        let mut bad = s.clone();
        bad.extra_args = vec![owned.into()];
        assert!(bad.validate().is_err(), "{owned} must be refused");
    }
}

#[test]
fn params_cli_sets_shows_and_unsets() {
    let root = tempfile::tempdir().unwrap();
    let alt = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_alt"))
            .arg("--data-dir")
            .arg(root.path())
            .args(args)
            .output()
            .unwrap()
    };
    assert!(
        alt(&[
            "init",
            "--model",
            "fixture",
            "--endpoint",
            "http://127.0.0.1:9/v1"
        ])
        .status
        .success()
    );
    let set = alt(&["params", "set", "seed", "42"]);
    assert!(
        set.status.success(),
        "{}",
        String::from_utf8_lossy(&set.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&set.stdout).unwrap();
    assert_eq!(report["effective_request_fields"]["seed"], 42);
    let refused = alt(&["params", "set", "extra_body", r#"{"model":"other"}"#]);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("model"));
    let runtime = alt(&["params", "set", "extra_args", "--port 1"]);
    assert!(!runtime.status.success());
    let unset = alt(&["params", "unset", "seed"]);
    let report: serde_json::Value = serde_json::from_slice(&unset.stdout).unwrap();
    assert!(report["effective_request_fields"].get("seed").is_none());
    assert!(
        report["owned_runtime_argv"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a == "--api-key-file")
    );
}
