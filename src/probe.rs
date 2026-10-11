//! A05 model capability probe: one bounded look at a connected model, then
//! labelled suggestions. Nothing is applied without the user's choice.
use crate::config::{Config, Preferences, Profile, Provider};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{path::Path, time::Duration};

/// Where a probe report for this exact model identity is saved.
pub fn report_path(root: &Path, p: &Profile) -> std::path::PathBuf {
    let identity = format!(
        "{:?}|{}|{}|{}",
        p.provider,
        p.endpoint,
        p.model,
        p.local_model.as_deref().unwrap_or("")
    );
    root.join("probes").join(format!(
        "{}.json",
        crate::project::digest(identity.as_bytes())
    ))
}

async fn get_json(client: &reqwest::Client, request: reqwest::RequestBuilder) -> Option<Value> {
    let _ = client;
    let response = request.send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let bytes = response.bytes().await.ok()?;
    (bytes.len() <= 4 * 1024 * 1024)
        .then(|| serde_json::from_slice(&bytes).ok())
        .flatten()
}

/// Native context and template facts the server reports, without inference.
async fn server_facts(client: &reqwest::Client, p: &Profile, key: Option<&str>) -> Value {
    let auth = |r: reqwest::RequestBuilder| match key {
        Some(k) => r.bearer_auth(k),
        None => r,
    };
    match p.provider {
        Provider::Ollama => {
            let request = auth(
                client
                    .post(format!("{}/api/show", p.endpoint.trim_end_matches('/')))
                    .json(&json!({"model": p.model})),
            );
            match get_json(client, request).await {
                Some(show) => {
                    let context = show["model_info"].as_object().and_then(|info| {
                        info.iter()
                            .find(|(k, _)| k.ends_with(".context_length"))
                            .and_then(|(_, v)| v.as_u64())
                    });
                    let capabilities: Vec<String> = show["capabilities"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|c| c.as_str().map(String::from))
                        .collect();
                    json!({"source":"/api/show","native_context":context,"thinking_template":capabilities.iter().any(|c| c == "thinking"),"tools_capability":capabilities.iter().any(|c| c == "tools"),"capabilities":capabilities})
                }
                None => {
                    json!({"source":"/api/show","native_context":null,"note":"Ollama did not describe this model"})
                }
            }
        }
        Provider::Openai => {
            let base = p.endpoint.trim_end_matches('/').trim_end_matches("/v1");
            match get_json(client, auth(client.get(format!("{base}/props")))).await {
                Some(props) => {
                    let template = props["chat_template"].as_str().unwrap_or("");
                    json!({"source":"/props (llama.cpp)","native_context":props["default_generation_settings"]["n_ctx"].as_u64().or(props["n_ctx"].as_u64()),"thinking_template":template.contains("enable_thinking") || template.contains("<think>"),"server":"llama.cpp"})
                }
                None => {
                    json!({"source":"none","native_context":null,"note":"This OpenAI-compatible server reports no native context; Alt keeps the configured value"})
                }
            }
        }
    }
}

/// Run the probe against an already reachable endpoint (owned runtimes are
/// started by the caller). Bounded to one model request and 90 seconds.
pub async fn probe(root: &Path, p: &Profile) -> Result<Value> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(90))
        .build()?;
    let key = p.key()?;
    let facts = server_facts(&client, p, key.as_deref()).await;
    let settings = p.effective_inference();
    let allowance = settings.output_tokens.min(512);
    // Observe the model's own default behaviour: no thinking or sampling choice.
    let mut probe_settings = crate::inference::Settings::legacy(p.context_tokens);
    probe_settings.output_parameter = settings.output_parameter;
    probe_settings.dialect = settings.dialect;
    let mut neutral = p.clone();
    neutral.inference = Some(probe_settings.clone());
    let tool = json!({"type":"function","function":{"name":"probe_echo","description":"Return the given text. Call it to answer.","parameters":{"type":"object","properties":{"text":{"type":"string"}},"required":["text"],"additionalProperties":false}}});
    let mut body = json!({"model":p.model,"messages":[{"role":"user","content":"Call the probe_echo tool with text set to ok. Do not answer in prose."}],"tools":[tool],"stream":false});
    probe_settings.apply(&neutral, &mut body)?;
    let field = match settings.output_parameter {
        crate::inference::OutputParameter::MaxTokens => "max_tokens",
        crate::inference::OutputParameter::MaxCompletionTokens => "max_completion_tokens",
    };
    body[field] = json!(allowance);
    let url = match p.provider {
        Provider::Openai => format!("{}/chat/completions", p.endpoint.trim_end_matches('/')),
        Provider::Ollama => format!("{}/v1/chat/completions", p.endpoint.trim_end_matches('/')),
    };
    let mut request = client.post(url).json(&body);
    if let Some(key) = &key {
        request = request.bearer_auth(key);
    }
    let started = std::time::Instant::now();
    let response = request
        .send()
        .await
        .context("The probe request could not reach the model")?;
    let status = response.status().as_u16();
    let bytes = response.bytes().await?;
    ensure!(
        bytes.len() <= 4 * 1024 * 1024,
        "Probe response exceeds 4 MiB"
    );
    let reply: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    ensure!(
        (200..300).contains(&status),
        "The model server answered the probe with HTTP {status}: {}",
        crate::display_text(
            &String::from_utf8_lossy(&bytes)
                .chars()
                .take(400)
                .collect::<String>()
        )
    );
    let message = &reply["choices"][0]["message"];
    let finish = reply["choices"][0]["finish_reason"]
        .as_str()
        .unwrap_or("unknown")
        .to_owned();
    let content = message["content"].as_str().unwrap_or("");
    let reasoning = ["reasoning_content", "reasoning", "thinking"]
        .iter()
        .any(|k| message[*k].as_str().is_some_and(|t| !t.trim().is_empty()))
        || content.contains("<think>");
    let tool_called = message["tool_calls"]
        .as_array()
        .is_some_and(|calls| calls.iter().any(|c| c["function"]["name"] == "probe_echo"));
    let observed = json!({"http_status":status,"finish_reason":finish,"tool_call":tool_called,"reasoning_seen":reasoning,"answer_text":!content.trim().is_empty(),"completion_tokens":reply["usage"]["completion_tokens"],"output_allowance":allowance,"seconds":started.elapsed().as_secs_f32()});
    let suggestions = suggest(p, &facts, &observed);
    let report = json!({"schema":1,"model":p.model,"provider":p.provider,"endpoint":p.endpoint,"server":facts,"observed":observed,"suggestions":suggestions,"scope":"One bounded request with the model's default thinking; suggestions are not applied until you choose one"});
    let path = report_path(root, p);
    crate::config::atomic_write(&path, &serde_json::to_vec_pretty(&report)?)?;
    Ok(report)
}

/// Labelled suggestions from observed facts. Pure; tested directly.
pub fn suggest(p: &Profile, facts: &Value, observed: &Value) -> Vec<Value> {
    let settings = p.effective_inference();
    let mut out = Vec::new();
    let reasoning = observed["reasoning_seen"] == true;
    let length = observed["finish_reason"] == "length";
    let tool = observed["tool_call"] == true;
    if reasoning && (length || !tool) {
        out.push(json!({"id":"thinking-off","label":"Turn thinking off","why":"The model reasoned and did not finish the tool call within the probe's allowance","change":{"thinking":"off"}}));
    } else if reasoning {
        out.push(json!({"id":"thinking-low","label":"Use low thinking","why":"The model reasons before acting; low keeps more of the allowance for actions","change":{"thinking":"low"}}));
    }
    if length {
        let larger = (settings.output_tokens.saturating_mul(2))
            .max(2048)
            .min(p.context_tokens / 2);
        if larger > settings.output_tokens {
            out.push(json!({"id":"more-output","label":format!("Raise the output allowance to {larger}"),"why":"The probe response stopped at its output limit","change":{"output_tokens":larger}}));
        }
    }
    if !tool && !reasoning {
        out.push(json!({"id":"compact-tools","label":"Use compact tools","why":"The model did not call the offered tool; smaller schemas help small models","change":{"tool_profile":"compact"}}));
    }
    if let Some(native) = facts["native_context"].as_u64()
        && native >= 2048
        && native < p.context_tokens as u64
    {
        out.push(json!({"id":"native-context","label":format!("Match the server context ({native})"),"why":format!("The server reports {native} native context tokens, less than the configured {}", p.context_tokens),"change":{"context_tokens":native}}));
    }
    out
}

/// Apply one saved suggestion to the named connection, after the user chose it.
pub fn apply(root: &Path, profile: Option<&str>, id: &str) -> Result<Value> {
    let mut config = Config::read(root)?;
    let name = config.profile(profile)?.0.to_owned();
    let entry = config
        .profiles
        .get_mut(&name)
        .context("Connection disappeared")?;
    let report: Value = serde_json::from_slice(
        &std::fs::read(report_path(root, entry))
            .context("Probe this model first; no saved probe matches this connection")?,
    )?;
    let suggestion = report["suggestions"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|s| s["id"] == id)
        .with_context(|| format!("The saved probe has no suggestion `{id}`"))?
        .clone();
    let change = &suggestion["change"];
    let mut settings = entry.effective_inference();
    if let Some(choice) = change["thinking"].as_str() {
        settings.thinking = choice.parse()?;
        settings.reasoning_tokens = None;
    }
    if let Some(tokens) = change["output_tokens"].as_u64() {
        settings.output_tokens = u32::try_from(tokens)?;
    }
    if let Some(context) = change["context_tokens"].as_u64() {
        entry.context_tokens = u32::try_from(context)?;
        settings.output_tokens = settings.output_tokens.min(entry.context_tokens / 2);
        settings.action_headroom = settings.action_headroom.min(settings.output_tokens);
    }
    if change["tool_profile"] == "compact" {
        let mut preferences = Preferences::load(root)?;
        preferences.tool_profile = crate::toolbox::ToolProfile::Compact;
        preferences.save(root)?;
    }
    settings.validate(entry)?;
    entry.inference = Some(settings);
    entry.validate()?;
    config.save(root)?;
    Ok(json!({"applied":suggestion,"connection":name}))
}

/// Probe a saved connection, starting and stopping Alt's own runtime for a
/// local model file.
pub async fn run(
    root: &Path,
    preferences: &Preferences,
    profile: &Profile,
    cancel: crate::models::Cancel,
) -> Result<Value> {
    let mut effective = profile.clone();
    let mut owned = if profile.local_model.is_some() {
        let runtime =
            crate::runtime::LocalRuntime::start(root, preferences, profile, cancel.clone(), |_| {})
                .await?;
        runtime.attach(&mut effective);
        Some(runtime)
    } else {
        None
    };
    let result = tokio::select! {
        result = probe(root, &effective) => result,
        _ = crate::models::cancelled(&cancel) => Err(anyhow::anyhow!("Probe cancelled")),
    };
    if let Some(runtime) = &mut owned {
        runtime.stop().await;
    }
    // Reports are keyed to the saved identity, not the per-launch endpoint.
    if let Ok(report) = &result
        && profile.local_model.is_some()
    {
        crate::config::atomic_write(
            &report_path(root, profile),
            &serde_json::to_vec_pretty(report)?,
        )?;
    }
    result
}
