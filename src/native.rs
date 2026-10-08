//! Actual native-tool probe and strict bounded stream reconstruction, shared by evaluation.
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path, time::Duration};

pub fn decode(raw: &[u8], stream: bool) -> Result<Value> {
    ensure!(raw.len() <= 1024 * 1024, "Probe response exceeds 1 MiB");
    if !stream {
        let v: Value = serde_json::from_slice(raw)?;
        ensure!(v.get("error").is_none(), "Provider returned an error");
        ensure!(
            v["choices"].as_array().is_some_and(|a| a.len() == 1)
                && v["choices"][0]["message"].is_object()
                && matches!(
                    v["choices"][0]["finish_reason"].as_str(),
                    Some("stop" | "tool_calls")
                ),
            "Provider did not return one completed native decision"
        );
        return Ok(v["choices"][0]["message"].clone());
    }
    let text = std::str::from_utf8(raw)?;
    let mut content = String::new();
    let mut reasoning = String::new();
    let mut calls: BTreeMap<u64, Value> = BTreeMap::new();
    let mut done = false;
    let mut finished = false;
    for line in text.lines() {
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data == "[DONE]" {
            done = true;
            continue;
        }
        ensure!(!done, "Data after stream completion");
        let value: Value = serde_json::from_str(data)?;
        ensure!(value.get("error").is_none(), "Provider stream error");
        ensure!(
            value["choices"]
                .as_array()
                .is_some_and(|rows| rows.len() <= 1
                    && rows
                        .first()
                        .is_none_or(|row| row["index"].as_u64().unwrap_or(0) == 0)),
            "Multiple or invalid native response choices"
        );
        let choice = &value["choices"][0];
        if !choice["finish_reason"].is_null() {
            ensure!(
                matches!(
                    choice["finish_reason"].as_str(),
                    Some("stop" | "tool_calls")
                ),
                "Provider ended before a complete native decision: {}",
                choice["finish_reason"]
            );
            finished = true;
        }
        let delta = &choice["delta"];
        if let Some(s) = delta["content"].as_str() {
            content.push_str(s);
        }
        if let Some(s) = delta["reasoning_content"]
            .as_str()
            .or_else(|| delta["reasoning"].as_str())
        {
            reasoning.push_str(s);
        }
        if let Some(rows) = delta["tool_calls"].as_array() {
            for part in rows {
                let index = part["index"]
                    .as_u64()
                    .context("Tool fragment lacks a valid index")?;
                ensure!(index < 16, "Too many native calls");
                let call = calls.entry(index).or_insert_with(
                    || json!({"id":"","type":"function","function":{"name":"","arguments":""}}),
                );
                if let Some(id) = part["id"].as_str() {
                    let old = call["id"].as_str().unwrap_or("");
                    ensure!(old.is_empty() || old == id, "Tool id changed mid-stream");
                    call["id"] = json!(id);
                }
                for field in ["name", "arguments"] {
                    if let Some(s) = part["function"][field].as_str() {
                        let mut previous =
                            call["function"][field].as_str().unwrap_or("").to_owned();
                        previous.push_str(s);
                        call["function"][field] = json!(previous);
                    }
                }
            }
        }
    }
    ensure!(
        done && finished,
        "Provider stream ended without completion evidence"
    );
    let mut message = json!({"role":"assistant","content":content});
    if !reasoning.is_empty() {
        message["reasoning_content"] = json!(reasoning);
    }
    if !calls.is_empty() {
        message["tool_calls"] = json!(calls.into_values().collect::<Vec<_>>());
    }
    Ok(message)
}

pub async fn probe(
    root: &Path,
    prefs: &crate::config::Preferences,
    profile: &crate::config::Profile,
    timeout: u64,
    force_tool: bool,
    cancel: crate::models::Cancel,
) -> Result<Value> {
    ensure!(
        profile.uncensored,
        "Live probes require explicit uncensored/abliterated selection"
    );
    ensure!(
        (1..=600).contains(&timeout),
        "Use a 1..600 second probe limit"
    );
    let id = uuid::Uuid::new_v4().to_string();
    let directory = root.join("evaluations").join(format!("native-{id}"));
    crate::config::private_dir(&directory)?;
    let mut report = json!({"schema":1,"id":id,"status":"running","model":profile.model,"scope":"Native calls and actual deterministic host execution; not broad reasoning or task reliability","timeout_seconds":timeout,"tool_choice":if force_tool{"required"}else{"auto"},"physical_gpu_tested":false,"steps":[]});
    let mut local = None;
    let mut effective = profile.clone();
    let started = std::time::Instant::now();
    let operation = async {
        if profile.local_model.is_some() {
            let runtime =
                crate::runtime::LocalRuntime::start(root, prefs, profile, cancel.clone(), |_| {})
                    .await?;
            effective.endpoint = runtime.endpoint.clone();
            local = Some(runtime);
        }
        let client = crate::runtime::http_client(&effective)
            .timeout(Duration::from_secs(timeout))
            .build()?;
        let template = if local.is_some() {
            let mut response = client
                .get(format!(
                    "{}/props",
                    effective.endpoint.trim_end_matches("/v1")
                ))
                .send()
                .await?
                .error_for_status()?;
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await? {
                ensure!(
                    bytes.len() + chunk.len() <= 2 * 1024 * 1024,
                    "Runtime properties exceed 2 MiB"
                );
                bytes.extend_from_slice(&chunk);
            }
            let props: Value = serde_json::from_slice(&bytes)?;
            report["runtime_properties"] = props.clone();
            report["observed_support"] = json!({"tools":props["chat_template_caps"]["supports_tools"],"native_tool_calls":props["chat_template_caps"]["supports_tool_calls"],"thinking_toggle_template":props["chat_template"].as_str().is_some_and(|s|s.contains("enable_thinking")),"requested_thinking_toggle":prefs.runtime.thinking,"scope":"Template/runtime metadata; actual call results are measured separately"});
            props["chat_template"].as_str().map(str::to_owned)
        } else {
            None
        };
        report["identity"] =
            crate::capability::identity(root, prefs, profile, template.as_deref())?;
        let tool = json!({"type":"function","function":{"name":"echo_probe","description":"Validate the supplied text and return a new host-generated observation. Read the actual returned text; it cannot be predicted from the input.","parameters":{"type":"object","properties":{"text":{"type":"string"}},"required":["text"],"additionalProperties":false}}});
        report["probe_tool_schema_sha256"] =
            json!(crate::project::digest(&serde_json::to_vec(&json!([tool]))?));
        let nonce = format!("ALT_NATIVE_{}", uuid::Uuid::new_v4());
        report["challenge_version"] = json!(2);
        let mut host_result = None;
        let mut messages = json!([{"role":"user","content":format!("Call echo_probe with text exactly {nonce}. After its result, report the exact text returned by the tool. The host creates a new value after your call; do not repeat the input. Use a native tool call.")}]);
        let mut body = json!({"model":profile.model,"messages":messages,"tools":[tool],"parallel_tool_calls":false,"stream":true});
        if force_tool {
            body["tool_choice"] = json!("required");
        }
        effective
            .effective_inference()
            .apply(&effective, &mut body)?;
        let url = match effective.provider {
            crate::config::Provider::Openai => format!(
                "{}/chat/completions",
                effective.endpoint.trim_end_matches('/')
            ),
            crate::config::Provider::Ollama => format!(
                "{}/v1/chat/completions",
                effective.endpoint.trim_end_matches('/')
            ),
        };
        let request = async |body: &Value, round: u32| -> Result<Vec<u8>> {
            let mut partial =
                std::fs::File::create(directory.join(format!("{round}-response.raw")))?;
            let mut req = client.post(&url).json(body);
            if let Some(key) = effective.key()? {
                req = req.bearer_auth(key);
            }
            let mut response = req.send().await?.error_for_status()?;
            let mut raw = Vec::new();
            while let Some(chunk) = response.chunk().await? {
                ensure!(
                    raw.len() + chunk.len() <= 1024 * 1024,
                    "Probe output exceeds 1 MiB"
                );
                std::io::Write::write_all(&mut partial, &chunk)?;
                raw.extend(chunk);
            }
            partial.sync_all()?;
            Ok(raw)
        };
        for round in 0..2 {
            crate::config::atomic_write(
                &directory.join(format!("{round}-request.json")),
                &serde_json::to_vec(&body)?,
            )?;
            let raw = tokio::select! {v=request(&body,round)=>v?,_=crate::models::cancelled(&cancel)=>anyhow::bail!("Native probe cancelled")};
            crate::config::atomic_write(&directory.join(format!("{round}-response.raw")), &raw)?;
            let message = decode(&raw, true)?;
            if round == 0 {
                let calls = message["tool_calls"]
                    .as_array()
                    .context("No native call was generated")?;
                ensure!(calls.len() == 1, "Expected exactly one native probe call");
                let call = &calls[0];
                ensure!(
                    call["function"]["name"] == "echo_probe",
                    "Unknown native tool name"
                );
                let args: Value = serde_json::from_str(
                    call["function"]["arguments"]
                        .as_str()
                        .context("Native arguments are not a string")?,
                )?;
                ensure!(
                    args.as_object().is_some_and(|o| o.len() == 1)
                        && args["text"].as_str() == Some(&nonce),
                    "Native arguments violate the probe contract"
                );
                let call_id = call["id"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .context("Native tool id missing")?;
                let returned = format!("ALT_HOST_{}", uuid::Uuid::new_v4());
                host_result = Some(returned.clone());
                let observation = json!({"role":"tool","tool_call_id":call_id,"content":returned});
                report["steps"].as_array_mut().unwrap().push(
                    json!({"native_call":call,"host_observation":observation,"executed":true}),
                );
                messages.as_array_mut().unwrap().push(message.clone());
                messages.as_array_mut().unwrap().push(observation);
                body["messages"] = messages.clone();
                body.as_object_mut().unwrap().remove("tool_choice");
            } else {
                let returned = host_result
                    .as_ref()
                    .context("Host result was not executed")?;
                ensure!(
                    message["tool_calls"].is_null(),
                    "Probe repeated a call instead of consuming its result"
                );
                ensure!(
                    message["content"]
                        .as_str()
                        .is_some_and(|s| s.contains(returned)),
                    "Final response did not use the actual host result"
                );
                report["steps"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"final_response":message,"host_result_used":true}));
            }
        }
        Ok::<(), anyhow::Error>(())
    };
    let result = tokio::time::timeout(Duration::from_secs(timeout), operation).await;
    if let Some(mut runtime) = local {
        runtime.stop().await;
    }
    report["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
    match result {
        Ok(Ok(())) => report["status"] = json!("passed"),
        Ok(Err(e)) => {
            report["status"] = json!("failed");
            report["error"] = json!(e.to_string());
        }
        Err(_) => {
            report["status"] = json!("timeout");
            report["error"] = json!("Whole probe allowance exhausted");
        }
    }
    crate::config::atomic_write(
        &directory.join("report.json"),
        &serde_json::to_vec_pretty(&report)?,
    )?;
    report["evidence_directory"] = json!(directory);
    Ok(report)
}
