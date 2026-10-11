//! A01: provider-aware reasoning control and reasoning-only cutoff recovery.
#[path = "fixtures/upstream.rs"]
mod upstream;

use alt_cli::{
    config::{Profile, Provider},
    inference::{Dialect, Relay, Settings, Stage, Thinking},
};
use serde_json::json;
use upstream::Reply;

fn profile(endpoint: &str, provider: Provider, settings: Settings) -> Profile {
    Profile {
        provider,
        endpoint: endpoint.into(),
        model: "fixture".into(),
        context_tokens: 8192,
        max_turns: 2,
        uncensored: true,
        api_key_env: None,
        local_model: None,
        inference: Some(settings),
        runtime_key: None,
    }
}

fn settings(thinking: Thinking, dialect: Dialect) -> Settings {
    let mut s = Settings::legacy(8192);
    s.output_tokens = 1024;
    s.action_headroom = 256;
    s.thinking = thinking;
    s.dialect = dialect;
    s
}

fn body_for(p: &Profile) -> serde_json::Value {
    let mut body =
        json!({"model":"fixture","messages":[],"stream":false,"reasoning_effort":"engine-default"});
    p.effective_inference().apply(p, &mut body).unwrap();
    body
}

#[test]
fn each_dialect_gets_its_own_reasoning_fields() {
    let url = "http://127.0.0.1:9/v1";
    // llama.cpp: template switch plus a budget that keeps the action headroom.
    let llama = profile(
        url,
        Provider::Openai,
        settings(Thinking::Low, Dialect::LlamaCpp),
    );
    let body = body_for(&llama);
    assert_eq!(
        body["chat_template_kwargs"],
        json!({"enable_thinking":true})
    );
    assert_eq!(body["reasoning_budget_tokens"], 192);
    assert!(body.get("reasoning_effort").is_none());
    let off = profile(
        url,
        Provider::Openai,
        settings(Thinking::Off, Dialect::LlamaCpp),
    );
    let body = body_for(&off);
    assert_eq!(
        body["chat_template_kwargs"],
        json!({"enable_thinking":false})
    );
    assert_eq!(body["reasoning_budget_tokens"], 0);
    // Ollama by provider: native think and reasoning_effort.
    let ollama = profile(
        "http://127.0.0.1:9",
        Provider::Ollama,
        settings(Thinking::Off, Dialect::Auto),
    );
    let body = body_for(&ollama);
    assert_eq!(body["think"], false);
    assert_eq!(body["reasoning_effort"], "none");
    assert!(body.get("chat_template_kwargs").is_none());
    // vLLM / SGLang / NIM: chat_template_kwargs for both common template names.
    let nim = profile(
        url,
        Provider::Openai,
        settings(Thinking::High, Dialect::ChatTemplate),
    );
    let body = body_for(&nim);
    assert_eq!(
        body["chat_template_kwargs"],
        json!({"enable_thinking":true,"thinking":true})
    );
    assert_eq!(body["reasoning_effort"], "high");
    // Generic OpenAI-compatible: reasoning_effort only.
    let openai = profile(
        url,
        Provider::Openai,
        settings(Thinking::Medium, Dialect::Auto),
    );
    let body = body_for(&openai);
    assert_eq!(body["reasoning_effort"], "medium");
    assert!(body.get("think").is_none() && body.get("chat_template_kwargs").is_none());
    // Default sends nothing, and removes an engine-chosen value.
    let default = profile(
        url,
        Provider::Openai,
        settings(Thinking::Default, Dialect::Auto),
    );
    let body = body_for(&default);
    assert!(body.get("reasoning_effort").is_none());
}

#[test]
fn budgets_are_only_for_llama_cpp_and_keep_headroom() {
    let url = "http://127.0.0.1:9/v1";
    let ollama = profile(
        "http://127.0.0.1:9",
        Provider::Ollama,
        settings(Thinking::Budget(100), Dialect::Auto),
    );
    assert!(ollama.validate().is_err());
    let over = profile(
        url,
        Provider::Openai,
        settings(Thinking::Budget(900), Dialect::LlamaCpp),
    );
    assert!(over.validate().is_err());
    let fits = profile(
        url,
        Provider::Openai,
        settings(Thinking::Budget(500), Dialect::LlamaCpp),
    );
    fits.validate().unwrap();
    assert_eq!(body_for(&fits)["reasoning_budget_tokens"], 500);
    for text in ["default", "off", "low", "medium", "high", "budget:512"] {
        let choice: Thinking = text.parse().unwrap();
        assert_eq!(String::from(choice), text);
    }
    assert!("sometimes".parse::<Thinking>().is_err());
}

#[tokio::test]
async fn reasoning_only_cutoff_is_detected_charged_and_recoverable_by_choice() {
    let cutoff = Reply::sse(&[
        json!({"choices":[{"delta":{"reasoning_content":"Let me think about the file layout…"},"finish_reason":null}]}),
        json!({"choices":[{"delta":{"reasoning_content":" still thinking"},"finish_reason":"length"}]}),
        json!({"choices":[],"usage":{"completion_tokens":1024}}),
    ]);
    let answer = Reply::sse(&[
        json!({"choices":[{"delta":{"content":"Done."},"finish_reason":"stop"}]}),
        json!({"choices":[],"usage":{"completion_tokens":3}}),
    ]);
    let server = upstream::start(vec![cutoff, answer]).await;
    let root = tempfile::tempdir().unwrap();
    let p = profile(
        &server.endpoint,
        Provider::Openai,
        settings(Thinking::High, Dialect::ChatTemplate),
    );
    let relay = Relay::start(root.path(), &p).await.unwrap();
    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", relay.endpoint);
    let request =
        json!({"model":"fixture","messages":[{"role":"user","content":"fix it"}],"stream":true});
    client
        .post(&url)
        .json(&request)
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let status = relay.status();
    assert_eq!(status.stage, Stage::ReasoningCutoff);
    assert_eq!(status.reasoning_cutoffs, 1);
    let receipt = std::fs::read_dir(&relay.evidence)
        .unwrap()
        .map(|f| f.unwrap().path())
        .find(|p| p.to_string_lossy().ends_with("-receipt.json"))
        .unwrap();
    let receipt: serde_json::Value =
        serde_json::from_slice(&std::fs::read(receipt).unwrap()).unwrap();
    assert_eq!(receipt["reasoning_cutoff"], true);
    assert_eq!(
        receipt["charged_generated_tokens"], 1024,
        "spent tokens stay charged"
    );
    assert_eq!(receipt["thinking"]["dialect"], "chat-template");
    assert_eq!(
        receipt["thinking"]["fields"]["chat_template_kwargs"]["enable_thinking"],
        true
    );
    // Nothing was resent automatically.
    assert_eq!(server.bodies().len(), 1);
    // The user's explicit choice applies to the rest of this connection.
    relay.set_thinking(Thinking::Off).unwrap();
    assert_eq!(relay.status().stage, Stage::Ready);
    client
        .post(&url)
        .json(&request)
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let bodies = server.bodies();
    assert_eq!(bodies.len(), 2);
    assert_eq!(
        bodies[1]["chat_template_kwargs"],
        json!({"enable_thinking":false,"thinking":false})
    );
    assert!(bodies[1].get("reasoning_effort").is_none());
    assert_eq!(relay.status().reasoning_cutoffs, 1);
    assert!(std::fs::read_dir(&relay.evidence).unwrap().any(|f| {
        f.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("thinking-")
    }));
}

#[test]
fn answers_and_tool_calls_are_not_cutoffs() {
    use alt_cli::inference::reasoning_cutoff;
    let tool = json!({"choices":[{"message":{"reasoning_content":"plan","tool_calls":[{"id":"1"}]},"finish_reason":"length"}]});
    assert!(!reasoning_cutoff(tool.to_string().as_bytes(), false));
    let text = json!({"choices":[{"message":{"reasoning":"plan","content":"partial answer"},"finish_reason":"length"}]});
    assert!(!reasoning_cutoff(text.to_string().as_bytes(), false));
    let stopped =
        json!({"choices":[{"message":{"reasoning":"plan","content":""},"finish_reason":"stop"}]});
    assert!(!reasoning_cutoff(stopped.to_string().as_bytes(), false));
    let cut =
        json!({"choices":[{"message":{"reasoning":"plan","content":""},"finish_reason":"length"}]});
    assert!(reasoning_cutoff(cut.to_string().as_bytes(), false));
}
