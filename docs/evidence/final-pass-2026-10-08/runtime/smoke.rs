use alt_cli::{config::{Preferences, Profile, Provider}, models, runtime};
use std::{path::Path, sync::{Arc, atomic::AtomicBool}, time::{Duration, Instant}};
use serde_json::json;
fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    if let Err(e) = rt.block_on(run()) { eprintln!("OWNED_LIVE_SMOKE_FAILED: {e:#}"); std::process::exit(1); }
}
async fn run() -> anyhow::Result<()> {
    let root = Path::new("/tmp/alt-runtime-live-smoke/state");
    let model = Path::new("/tmp/alt-native-requalification/Josiefied-Qwen2.5-7B-Instruct-abliterated.Q4_K_M.gguf");
    let signal = Arc::new(AtomicBool::new(false));
    let artifact = models::import(root, model, true, signal.clone(), |_| {}).await?;
    anyhow::ensure!(artifact.sha256 == "d7d626d96cc2d3567e8266101b4211b17634012dd99a0c9b28f475ce4eb620b6", "Checkpoint digest mismatch");
    let mut prefs = Preferences { runtime_path: Some("/workspace/.alt-tools/llama-b11429/llama-b11429/llama-server".into()), ..Preferences::default() };
    prefs.runtime.threads = 2;
    prefs.runtime.startup_timeout_secs = 180;
    let mut profile = Profile {provider: Provider::Openai, endpoint:"http://127.0.0.1:1/v1".into(), model:artifact.name.clone(), context_tokens:4096, max_turns:2, uncensored:true, api_key_env:None, local_model:Some(artifact.id), inference:None};
    let start = Instant::now();
    let mut owned = runtime::LocalRuntime::start(root, &prefs, &profile, signal, |_| {}).await?;
    let load_ms = start.elapsed().as_millis();
    let pid = owned.pid().unwrap();
    profile.endpoint = owned.endpoint.clone();
    let client = runtime::http_client(&profile).timeout(Duration::from_secs(90)).build()?;
    let endpoint = profile.endpoint.clone();
    let checks = async {
        let without_path = endpoint.split('/').take(3).collect::<Vec<_>>().join("/");
        let unauthorized = client.get(format!("{without_path}/health")).send().await?;
        anyhow::ensure!(unauthorized.status().as_u16()==403, "Unqualified private proxy access was allowed");
        owned.check_warm_identity()?;
        let started = Instant::now();
        let response = client.post(format!("{endpoint}/chat/completions")).json(&json!({"model":profile.model,"messages":[{"role":"user","content":"Reply with exactly the single word READY."}],"max_tokens":24,"temperature":0.0,"stream":false})).send().await?.error_for_status()?;
        let body = response.bytes().await?;
        anyhow::ensure!(body.len() <= 65536, "Unexpected response size");
        let value: serde_json::Value = serde_json::from_slice(&body)?;
        anyhow::ensure!(value["choices"][0]["message"]["content"].as_str().is_some_and(|s| !s.is_empty()), "No generated content");
        anyhow::ensure!(value["usage"]["completion_tokens"].as_u64().is_some_and(|n| n>0 && n<=24), "Missing or out-of-budget completion usage");
        Ok::<_,anyhow::Error>((value, started.elapsed().as_millis()))
    }.await;
    let observation = owned.offload_observation();
    let runtime_log = owned.log_path().to_owned();
    owned.stop().await;
    let reaped = !Path::new(&format!("/proc/{pid}")).exists();
    let receipt = match &checks {
        Ok((response, generation_ms)) => json!({"status":"passed", "checkpoint":artifact.name,"sha256":artifact.sha256,"runtime":"llama.cpp b11429","context_tokens":4096,"threads":2,"batch":128,"requested_gpu_layers":0,"load_ms":load_ms,"generation_ms":generation_ms,"request_max_tokens":24,"load_deadline_secs":180,"generation_deadline_secs":90,"inherited_proxy_test":"HTTP_PROXY and ALL_PROXY point at closed localhost port; NO_PROXY empty", "transport":"llama-server private Unix socket through held secret-path loopback proxy","unqualified_proxy_access_status":403,"actual_offload":observation,"owned_pid_reaped":reaped,"response":response,"scope":"CPU lifecycle and bounded generation, not task quality or GPU compatibility"}),
        Err(e) => json!({"status":"failed","error":format!("{e:#}"),"owned_pid_reaped":reaped}),
    };
    std::fs::copy(runtime_log, "/tmp/alt-runtime-live-smoke/runtime.log")?;
    std::fs::write("/tmp/alt-runtime-live-smoke/receipt.json",serde_json::to_vec_pretty(&receipt)?)?;
    println!("{}",serde_json::to_string_pretty(&receipt)?);
    checks?;
    anyhow::ensure!(reaped, "Owned runtime leader not reaped");
    Ok(())
}
