//! Explicit measured compatibility, separate from coding-quality assertions.
use crate::{
    config::{Preferences, Profile, Provider},
    models::{self, Cancel},
    runtime,
};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
pub async fn run(
    root: &Path,
    preferences: &Preferences,
    profile: &Profile,
    cancel: Cancel,
) -> Result<Value> {
    ensure!(
        profile.uncensored,
        "Benchmark requires an explicitly selected uncensored/abliterated checkpoint"
    );
    let id = uuid::Uuid::new_v4().to_string();
    let detected = crate::hardware::inspect().await;
    let mut report = json!({"id":id,"alt_version":env!("CARGO_PKG_VERSION"),"model":profile.model,"provider":profile.provider,"context":profile.context_tokens,"requested_context":profile.context_tokens,"observed_native_context":null,"settings":preferences.runtime,"hardware":detected,"gpu_measurement":"Unavailable unless a local owned runtime and nvidia-smi report process allocation","error":null,"task_completion":"Not measured by generation throughput; use the live project acceptance suite"});
    let mut local = None;
    let mut effective = profile.clone();
    let started = Instant::now();
    let samples = Arc::new(std::sync::Mutex::new((0u64, 0u64)));
    let done = Arc::new(AtomicBool::new(false));
    let mut monitor = None;
    let operation = async {
        if let Some(model) = profile.local_model.as_ref() {
            report["artifact"] = serde_json::to_value(models::artifact(root, model)?)?;
            let server = runtime::LocalRuntime::start_observed(
                root,
                preferences,
                profile,
                cancel.clone(),
                |_| {},
                |pid| {
                    monitor = Some(sample_runtime(pid, samples.clone(), done.clone()));
                },
            )
            .await?;
            effective.endpoint = server.endpoint.clone();
            report["load_and_integrity_seconds"] = json!(started.elapsed().as_secs_f64());
            report["runtime_log"] = json!(server.log_path());
            local = Some(server);
        } else {
            report["artifact"] =
                json!({"external_model_id":profile.model,"weights_verified_by_alt":false});
            report["load_and_integrity_seconds"] = Value::Null;
        }
        let mut template = None;
        if local.is_some() {
            let url = format!(
                "{}/props",
                effective
                    .endpoint
                    .trim_end_matches('/')
                    .trim_end_matches("/v1")
            );
            let props_client = runtime::http_client(&effective)
                .timeout(Duration::from_secs(5))
                .build()?;
            let props = async {
                let mut response = props_client.get(url).send().await?.error_for_status()?;
                let mut bytes = Vec::new();
                while let Some(chunk) = response.chunk().await? {
                    bytes.extend_from_slice(&chunk);
                    ensure!(
                        bytes.len() <= 2 * 1024 * 1024,
                        "Runtime properties exceed 2 MiB"
                    );
                }
                Ok::<Value, anyhow::Error>(serde_json::from_slice(&bytes)?)
            }
            .await;
            match props {
                Ok(value) => {
                    template = value["chat_template"].as_str().map(str::to_owned);
                    report["observed_native_context"] =
                        value["default_generation_settings"]["n_ctx"].clone();
                    report["runtime_properties"] = value;
                }
                Err(e) => report["runtime_properties_error"] = json!(format!("{e:#}")),
            }
        }
        let data = root.to_path_buf();
        let preferences_copy = preferences.clone();
        let profile_copy = profile.clone();
        report["configuration_identity"] = tokio::task::spawn_blocking(move || {
            crate::capability::identity(
                &data,
                &preferences_copy,
                &profile_copy,
                template.as_deref(),
            )
        })
        .await??;
        let mut trials = Vec::new();
        let client = runtime::http_client(&effective)
            .timeout(Duration::from_secs(180))
            .build()?;
        for n in 0..3 {
            if let Some(runtime) = local.as_mut() {
                runtime.check_warm_identity()?;
            }
            let mut body = json!({"model":effective.model,"messages":[{"role":"user","content":"Write a short numbered list of five practical steps for testing a small software project."}],"stream":true,"temperature":0,"max_tokens":160});
            let url = if effective.provider == Provider::Ollama {
                body["options"] =
                    json!({"temperature":0,"num_ctx":effective.context_tokens,"num_predict":160});
                format!("{}/api/chat", effective.endpoint.trim_end_matches('/'))
            } else {
                body["stream_options"] = json!({"include_usage":true});
                format!(
                    "{}/chat/completions",
                    effective.endpoint.trim_end_matches('/')
                )
            };
            let mut request = client.post(url).json(&body);
            if let Some(key) = effective.key()? {
                request = request.bearer_auth(key);
            }
            let start = Instant::now();
            let response = request.send().await?.error_for_status()?;
            let response_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|h| h.to_str().ok())
                .unwrap_or("")
                .to_string();
            let mut bytes = Vec::new();
            let mut pending = Vec::new();
            let mut first_token = None;
            let mut value = Value::Null;
            let streaming =
                response_type.contains("event-stream") || response_type.contains("ndjson");
            use futures_util::StreamExt;
            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                ensure!(
                    bytes.len() + chunk.len() <= 1024 * 1024,
                    "Benchmark response too large"
                );
                bytes.extend_from_slice(&chunk);
                if streaming {
                    pending.extend_from_slice(&chunk);
                    while let Some(end) = pending.iter().position(|b| *b == b'\n') {
                        let line: Vec<_> = pending.drain(..=end).collect();
                        let line = std::str::from_utf8(&line)?.trim();
                        let data = if response_type.contains("event-stream") {
                            line.strip_prefix("data:").map(str::trim)
                        } else {
                            Some(line)
                        };
                        if let Some(data) = data.filter(|s| !s.is_empty() && *s != "[DONE]") {
                            let frame: Value = serde_json::from_str(data)?;
                            let delta = &frame["choices"][0]["delta"];
                            if first_token.is_none()
                                && [
                                    delta.get("content"),
                                    delta.get("reasoning_content"),
                                    delta.get("reasoning"),
                                    frame["message"].get("content"),
                                    frame["message"].get("thinking"),
                                ]
                                .into_iter()
                                .flatten()
                                .any(|v| v.as_str().is_some_and(|s| !s.is_empty()))
                            {
                                first_token = Some(start.elapsed().as_secs_f64());
                            }
                            if frame.get("usage").is_some_and(|v| !v.is_null())
                                || frame.get("eval_count").is_some()
                                || value.is_null()
                            {
                                value = frame;
                            }
                        }
                    }
                }
            }
            if !streaming {
                value = serde_json::from_slice(&bytes)?;
            }
            let seconds = start.elapsed().as_secs_f64();
            let tokens = if effective.provider == Provider::Ollama {
                value["eval_count"].as_u64()
            } else {
                value["usage"]["completion_tokens"].as_u64()
            };
            ensure!(
                tokens.is_some_and(|t| t > 0),
                "Runtime did not report generated token usage"
            );
            let decode_rate = value["timings"]["predicted_per_second"]
                .as_f64()
                .or_else(|| {
                    value["eval_duration"]
                        .as_f64()
                        .filter(|n| *n > 0.0)
                        .map(|n| tokens.unwrap() as f64 * 1e9 / n)
                });
            let prompt_rate = value["timings"]["prompt_per_second"].as_f64().or_else(|| {
                value["prompt_eval_count"]
                    .as_f64()
                    .zip(value["prompt_eval_duration"].as_f64())
                    .filter(|(_, n)| *n > 0.0)
                    .map(|(count, ns)| count * 1e9 / ns)
            });
            trials.push(json!({"trial":n+1,"runtime_state":if local.is_none(){"external runtime lifecycle unmeasured"}else if n==0{"cold: first request after owned load"}else{"warm: same owned process, file identities checked"},"actual_request":body,"time_to_first_generated_token_seconds":first_token,"ttft_scope":if streaming{"Observed first nonempty content/reasoning stream field"}else{"Unmeasured: server returned a non-streaming body"},"seconds_including_prompt_processing":seconds,"generated_tokens":tokens,"tokens_per_wall_second":tokens.unwrap() as f64/seconds,"reported_decode_tokens_per_second":decode_rate,"reported_prompt_tokens_per_second":prompt_rate,"runtime_timings":value.get("timings"),"ollama_eval_duration_ns":value.get("eval_duration"),"response":value,"raw_response":String::from_utf8_lossy(&bytes)}));
            report["trials"] = json!(trials);
        }
        Ok(())
    };
    let operation: Result<()> = tokio::select! {
        result = operation => result,
        _ = models::cancelled(&cancel) => Err(anyhow::anyhow!("Benchmark cancelled")),
    };
    done.store(true, Ordering::Relaxed);
    if let Some(monitor) = monitor {
        let _ = monitor.await;
    }
    if let Some(runtime) = &mut local {
        report["observed_offload"] = runtime.offload_observation();
        runtime.stop().await;
        report["owned_runtime_stopped"] = json!(true);
    }
    let (ram, vram) = *samples.lock().expect("stats");
    report["sampled_peak_runtime_rss_bytes"] = if ram > 0 { json!(ram) } else { Value::Null };
    report["ram_measurement"] = json!(
        "Sampled owned runtime from process spawn through load and generation. Sampling may miss short peaks; external servers are not measured. Null means unavailable, not zero memory use."
    );
    report["sampled_peak_runtime_vram_bytes"] = if vram > 0 { json!(vram) } else { Value::Null };
    if let Err(error) = operation {
        report["error"] = json!(format!("{error:#}"));
        report["recovery"] = json!(
            "The owned runtime was stopped. Reduce context/batch/layers or choose another explicitly selected model and rerun; no automatic substitution occurred."
        );
    }
    let path = root
        .join("evaluations")
        .join(format!("benchmark-{id}.json"));
    crate::config::atomic_write(&path, &serde_json::to_vec_pretty(&report)?)?;
    Ok(report)
}

/// The selected managed llama.cpp endpoint; no model substitution or token guessing.
pub async fn token_count(profile: &Profile, content: &str) -> Result<usize> {
    let url = format!(
        "{}/tokenize",
        profile
            .endpoint
            .trim_end_matches('/')
            .trim_end_matches("/v1")
    );
    let client = runtime::http_client(profile)
        .timeout(Duration::from_secs(2))
        .build()?;
    let mut request = client
        .post(url)
        .json(&json!({"content":content,"add_special":false}));
    if let Some(key) = profile.key()? {
        request = request.bearer_auth(key);
    }
    let response = request.send().await?.error_for_status()?;
    let mut bytes = Vec::new();
    use futures_util::StreamExt;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        bytes.extend_from_slice(&chunk?);
        ensure!(bytes.len() < 1024 * 1024, "Tokenizer response too large");
    }
    let value: Value = serde_json::from_slice(&bytes)?;
    let tokens = value["tokens"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Tokenizer did not report tokens"))?;
    Ok(tokens.len())
}

fn sample_runtime(
    pid: u32,
    stats: Arc<std::sync::Mutex<(u64, u64)>>,
    stop: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while !stop.load(Ordering::Relaxed) {
            let rss = std::fs::read_to_string(format!("/proc/{pid}/status"))
                .ok()
                .and_then(|s| {
                    s.lines()
                        .find(|l| l.starts_with("VmRSS:"))
                        .and_then(|l| l.split_whitespace().nth(1))
                        .and_then(|n| n.parse::<u64>().ok())
                })
                .unwrap_or(0)
                * 1024;
            let gpu = crate::process::bounded_output(
                tokio::process::Command::new("nvidia-smi").args([
                    "--query-compute-apps=pid,used_memory",
                    "--format=csv,noheader,nounits",
                ]),
                Duration::from_millis(500),
                64 * 1024,
            )
            .await
            .ok()
            .filter(|r| r.status.success())
            .map(|r| {
                String::from_utf8_lossy(&r.stdout)
                    .lines()
                    .filter_map(|l| l.split_once(','))
                    .filter(|(p, _)| p.trim().parse::<u32>().ok() == Some(pid))
                    .filter_map(|(_, n)| n.trim().parse::<u64>().ok())
                    .sum::<u64>()
                    * 1024
                    * 1024
            })
            .unwrap_or(0);
            {
                let mut s = stats.lock().expect("stats");
                s.0 = s.0.max(rss);
                s.1 = s.1.max(gpu);
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
}
