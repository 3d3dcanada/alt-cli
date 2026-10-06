//! Repeatable throughput and owned-runtime lifecycle qualification. Never changes preferences.
use crate::{
    config::{Preferences, Profile},
    models::{self, Cancel},
    runtime::LocalRuntime,
};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

async fn lifecycle(
    root: &Path,
    prefs: &Preferences,
    profile: &Profile,
    cancel: Cancel,
) -> Result<Value> {
    if profile.local_model.is_none() {
        return Ok(
            json!({"status":"unmeasured","reason":"External runtime ownership belongs to the operator; Alt does not stop it"}),
        );
    }
    let mut runtime = LocalRuntime::start(root, prefs, profile, cancel.clone(), |_| {}).await?;
    let pid = runtime.pid();
    let url = format!(
        "{}/chat/completions",
        runtime.endpoint.trim_end_matches('/')
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()?;
    let request = async {
        let mut response=client.post(url).json(&json!({"model":profile.model,"messages":[{"role":"user","content":"Write a long numbered list of practical software tests."}],"stream":true,"max_tokens":2048})).send().await?.error_for_status()?;
        let mut pending = Vec::new();
        let mut received = 0usize;
        while let Some(chunk) = response.chunk().await? {
            received += chunk.len();
            ensure!(
                received <= 1024 * 1024,
                "Stream did not produce a generated token within 1 MiB"
            );
            pending.extend_from_slice(&chunk);
            while let Some(end) = pending.windows(2).position(|p| p == b"\n\n") {
                let frame: Vec<_> = pending.drain(..end + 2).collect();
                for line in std::str::from_utf8(&frame)?.lines() {
                    if let Some(data) = line.strip_prefix("data:") {
                        if data.trim() == "[DONE]" {
                            anyhow::bail!("Generation ended before a nonempty token");
                        }
                        let value: Value = serde_json::from_str(data.trim())?;
                        let delta = &value["choices"][0]["delta"];
                        if ["content", "reasoning_content", "reasoning"]
                            .iter()
                            .any(|key| delta[key].as_str().is_some_and(|s| !s.is_empty()))
                        {
                            return Ok::<_, anyhow::Error>(received);
                        }
                    }
                }
            }
        }
        anyhow::bail!("No generated token before stream closed")
    };
    let first = tokio::select! {r=request=>r,_=models::cancelled(&cancel)=>Err(anyhow::anyhow!("Qualification cancelled"))};
    let started = Instant::now();
    runtime.stop().await;
    let stop_ms = started.elapsed().as_millis();
    let stopped = pid.is_none_or(|p| !Path::new(&format!("/proc/{p}")).exists());
    ensure!(stopped, "Owned runtime remained alive after stop");
    let first_bytes = first?;
    ensure!(
        !cancel.load(std::sync::atomic::Ordering::Relaxed),
        "Qualification cancelled after owned runtime stopped"
    );
    let mut restarted = LocalRuntime::start(root, prefs, profile, cancel, |_| {}).await?;
    let health_url = format!(
        "{}/health",
        restarted
            .endpoint
            .trim_end_matches('/')
            .trim_end_matches("/v1")
    );
    let health = client
        .get(health_url)
        .send()
        .await
        .and_then(|r| r.error_for_status());
    let restarted_pid = restarted.pid();
    restarted.stop().await;
    health?;
    ensure!(
        restarted_pid.is_none_or(|p| !Path::new(&format!("/proc/{p}")).exists()),
        "Restarted runtime remained alive after cleanup"
    );
    Ok(
        json!({"status":"passed","stream_bytes_before_cancellation":first_bytes,"generated_token_observed":true,"cancel_method":"Drop active HTTP stream, stop and reap owned runtime","stop_acknowledgement_ms":stop_ms,"owned_pid_reaped":true,"restart_health_passed":true,"restart_cleanup_passed":true}),
    )
}

pub async fn run(
    root: &Path,
    prefs: &Preferences,
    profile: &Profile,
    contexts: &[u32],
    repeats: usize,
    cancel: Cancel,
) -> Result<Value> {
    ensure!(
        profile.uncensored,
        "Select an explicit uncensored/abliterated model; qualification never substitutes one"
    );
    ensure!(
        !contexts.is_empty() && contexts.len() <= 16 && (1..=10).contains(&repeats),
        "Use 1–16 contexts and 1–10 repeats"
    );
    ensure!(
        contexts.iter().all(|c| (2048..=1_048_576).contains(c)),
        "Contexts must be 2048–1048576"
    );
    let id = uuid::Uuid::new_v4().to_string();
    let path = root
        .join("evaluations")
        .join(format!("qualification-{id}.json"));
    let mut report = json!({"schema":1,"id":id,"status":"running","contexts":contexts,"repeats":repeats,"rows":[],"guidance":[],"scope":"Measured generation and owned-runtime lifecycle, not coding reliability. Remote context is requested; only runtime-reported context is measured. Resource-limited CPU tests are not physical GPU tests."});
    crate::config::atomic_write(&path, &serde_json::to_vec_pretty(&report)?)?;
    for &context in contexts {
        for repeat in 1..=repeats {
            if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            let mut selected = profile.clone();
            selected.context_tokens = context;
            let benchmark = crate::benchmark::run(root, prefs, &selected, cancel.clone()).await;
            let measurement = match benchmark {
                Ok(v) => v,
                Err(e) => json!({"error":format!("{e:#}")}),
            };
            let lifecycle = if measurement["error"].is_null()
                && !cancel.load(std::sync::atomic::Ordering::Relaxed)
            {
                match lifecycle(root, prefs, &selected, cancel.clone()).await {
                    Ok(v) => v,
                    Err(e) => json!({"status":"failed","error":format!("{e:#}")}),
                }
            } else {
                json!({"status":"unmeasured","reason":"Generation failed or cancelled"})
            };
            let pass = measurement["error"].is_null() && lifecycle["status"] == "passed";
            report["rows"].as_array_mut().expect("rows").push(json!({"context":context,"repeat":repeat,"generation_passed":measurement["error"].is_null(),"owned_lifecycle_passed":lifecycle["status"]=="passed","measurement":measurement,"lifecycle":lifecycle}));
            report["guidance"].as_array_mut().expect("guidance").push(json!({"context":context,"repeat":repeat,"text":if pass {"Generation, cancellation cleanup and restart passed on this measured computer. Review coding-task results before selecting this configuration."}else{"Not qualified for owned operation by this attempt. Inspect the error or complete external lifecycle checks before relying on it."}}));
            crate::config::atomic_write(&path, &serde_json::to_vec_pretty(&report)?)?;
        }
    }
    report["status"] = json!(if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        "cancelled"
    } else {
        "completed"
    });
    report["saved_report"] = json!(path);
    crate::config::atomic_write(&path, &serde_json::to_vec_pretty(&report)?)?;
    Ok(report)
}
pub fn fresh_cancel() -> Cancel {
    Arc::new(AtomicBool::new(false))
}

/// Human-readable guidance keeps timing and behavioral acceptance separate.
pub fn summary(report: &Value) -> String {
    let mut text = format!(
        "Measurement status: {}\n\n",
        report["status"].as_str().unwrap_or("unknown")
    );
    if let Some(rows) = report["rows"].as_array() {
        for row in rows {
            let m = &row["measurement"];
            let speeds: Vec<f64> = m["trials"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|t| t["tokens_per_wall_second"].as_f64())
                .collect();
            let speed = if speeds.is_empty() {
                "unavailable".into()
            } else {
                format!(
                    "{:.1} tokens/second (includes prompt processing)",
                    speeds.iter().sum::<f64>() / speeds.len() as f64
                )
            };
            let ram = m["sampled_peak_runtime_rss_bytes"]
                .as_u64()
                .map(crate::models::human_bytes)
                .unwrap_or_else(|| "unmeasured".into());
            let vram = m["sampled_peak_runtime_vram_bytes"]
                .as_u64()
                .map(crate::models::human_bytes)
                .unwrap_or_else(|| "unmeasured".into());
            text.push_str(&format!("{} requested context · attempt {}\nGeneration: {} · {}\nPeak sampled runtime RAM: {ram}; VRAM: {vram}\nStop and restart: {}\n",row["context"],row["repeat"],if row["generation_passed"]==true {"passed"}else{"failed"},speed,row["lifecycle"]["status"].as_str().unwrap_or("unmeasured")));
            if let Some(error) = m["error"].as_str().or(row["lifecycle"]["error"].as_str()) {
                text.push_str(&format!("What happened: {error}\n"));
            }
            text.push('\n');
        }
    }
    text.push_str("These measurements describe this computer and selected runtime. They do not establish coding reliability or fit on a different GPU. Settings were not changed.\n");
    if let Some(path) = report["saved_report"].as_str() {
        text.push_str(&format!("Full evidence: {path}"));
    }
    text
}
pub fn history(root: &Path, model: &str) -> Result<String> {
    let dir = root.join("evaluations");
    if !dir.is_dir() {
        return Ok("No qualification reports yet. Run a measurement first.".into());
    }
    let mut entries = std::fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| std::cmp::Reverse(e.metadata().and_then(|m| m.modified()).ok()));
    let mut text = String::new();
    let mut count = 0;
    for entry in entries {
        let path = entry.path();
        if !path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("qualification-"))
        {
            continue;
        }
        ensure!(
            entry.metadata()?.len() <= 16 * 1024 * 1024,
            "Qualification report exceeds 16 MiB"
        );
        let value: Value = serde_json::from_slice(&std::fs::read(path)?)?;
        if value["rows"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|r| r["measurement"]["model"] == model))
        {
            text.push_str(&summary(&value));
            text.push_str("\n\n");
            count += 1;
            if count == 8 {
                break;
            }
        }
    }
    if text.is_empty() {
        text = "No saved measurements for this model. Run qualification first.".into();
    }
    Ok(text)
}
