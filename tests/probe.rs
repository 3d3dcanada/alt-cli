//! A05: bounded model probe with labelled suggestions per server type.
#[path = "fixtures/upstream.rs"]
mod upstream;

use alt_cli::{
    config::{Config, Profile, Provider},
    probe,
};
use serde_json::json;
use upstream::Reply;

fn profile(endpoint: &str, provider: Provider) -> Profile {
    Profile {
        provider,
        endpoint: endpoint.into(),
        model: "fixture".into(),
        context_tokens: 16384,
        max_turns: 4,
        uncensored: true,
        api_key_env: None,
        local_model: None,
        inference: None,
        runtime_key: None,
    }
}

fn ids(report: &serde_json::Value) -> Vec<String> {
    report["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn llama_cpp_reasoning_cutoff_suggests_thinking_off_more_output_and_native_context() {
    let server = upstream::start(vec![
        Reply::json(json!({"default_generation_settings":{"n_ctx":8192},"chat_template":"{% if enable_thinking %}<think>{% endif %}"})),
        Reply::json(json!({"choices":[{"message":{"role":"assistant","content":"","reasoning_content":"Let me consider the tool…"},"finish_reason":"length"}],"usage":{"completion_tokens":512}})),
    ])
    .await;
    let root = tempfile::tempdir().unwrap();
    let p = profile(&server.endpoint, Provider::Openai);
    Config::create(root.path(), p.clone()).unwrap();
    let report = probe::probe(root.path(), &p).await.unwrap();
    assert_eq!(report["server"]["native_context"], 8192);
    assert_eq!(report["server"]["thinking_template"], true);
    assert_eq!(report["observed"]["reasoning_seen"], true);
    assert_eq!(
        ids(&report),
        ["thinking-off", "more-output", "native-context"]
    );
    let requests = server.requests.lock().unwrap().clone();
    assert!(requests[0].0.starts_with("GET /props"));
    assert_eq!(requests[1].1["tools"][0]["function"]["name"], "probe_echo");
    assert_eq!(requests[1].1["max_tokens"], 512);
    assert!(
        requests[1].1.get("chat_template_kwargs").is_none(),
        "probe observes defaults"
    );
    assert_eq!(requests.len(), 2, "exactly one model request");
    // Nothing changed until the user chose a suggestion.
    let before = Config::read(root.path()).unwrap();
    assert_eq!(before.profiles["local"].context_tokens, 16384);
    probe::apply(root.path(), None, "thinking-off").unwrap();
    probe::apply(root.path(), None, "native-context").unwrap();
    let after = Config::read(root.path()).unwrap();
    let saved = &after.profiles["local"];
    assert_eq!(
        saved.effective_inference().thinking,
        alt_cli::inference::Thinking::Off
    );
    assert_eq!(saved.context_tokens, 8192);
    assert_eq!(saved.model, "fixture", "never a different model");
    assert!(probe::apply(root.path(), None, "nonexistent").is_err());
}

#[tokio::test]
async fn ollama_reports_capabilities_and_a_working_tool_call_needs_no_change() {
    let server = upstream::start(vec![
        Reply::json(json!({"capabilities":["completion","tools"],"model_info":{"qwen3.context_length":32768}})),
        Reply::json(json!({"choices":[{"message":{"role":"assistant","content":null,"tool_calls":[{"id":"1","type":"function","function":{"name":"probe_echo","arguments":"{\"text\":\"ok\"}"}}]},"finish_reason":"tool_calls"}],"usage":{"completion_tokens":12}})),
    ])
    .await;
    let root = tempfile::tempdir().unwrap();
    let endpoint = server.endpoint.trim_end_matches("/v1").to_owned();
    let p = profile(&endpoint, Provider::Ollama);
    let report = probe::probe(root.path(), &p).await.unwrap();
    assert_eq!(report["server"]["source"], "/api/show");
    assert_eq!(report["server"]["native_context"], 32768);
    assert_eq!(report["server"]["tools_capability"], true);
    assert_eq!(report["observed"]["tool_call"], true);
    assert!(ids(&report).is_empty());
    let requests = server.requests.lock().unwrap().clone();
    assert!(requests[0].0.starts_with("POST /api/show"));
    assert!(requests[1].0.starts_with("POST /v1/chat/completions"));
    assert!(probe::report_path(root.path(), &p).exists());
}

#[tokio::test]
async fn an_openai_compatible_server_without_tool_calls_gets_compact_tools() {
    let server = upstream::start(vec![
        Reply::status(404, r#"{"error":"no props"}"#),
        Reply::json(json!({"choices":[{"message":{"role":"assistant","content":"ok"},"finish_reason":"stop"}],"usage":{"completion_tokens":2}})),
    ])
    .await;
    let root = tempfile::tempdir().unwrap();
    let p = profile(&server.endpoint, Provider::Openai);
    let report = probe::probe(root.path(), &p).await.unwrap();
    assert!(report["server"]["native_context"].is_null());
    assert_eq!(report["observed"]["tool_call"], false);
    assert_eq!(ids(&report), ["compact-tools"]);
}
