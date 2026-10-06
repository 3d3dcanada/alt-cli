//! Hardware guidance separates estimated fit, runtime reachability, and observed tool use.
use crate::{
    config::{Preferences, Profile, Provider},
    models, runtime,
};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub os: String,
    pub architecture: String,
    pub libc: Option<String>,
    pub ram: u64,
    pub available_ram: u64,
    pub cpu_threads: usize,
    pub cpu_flags: Vec<String>,
    pub gpu: String,
    pub driver: String,
    pub isolation: crate::sandbox::Isolation,
    pub profiles: Vec<Choice>,
    pub notes: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Choice {
    pub name: String,
    pub context: u32,
    pub model_hint: String,
    pub estimated_weight_budget: u64,
    pub explanation: String,
}
pub fn glibc() -> Option<String> {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        unsafe extern "C" {
            fn gnu_get_libc_version() -> *const std::ffi::c_char;
        }
        let value = unsafe { std::ffi::CStr::from_ptr(gnu_get_libc_version()) };
        Some(value.to_string_lossy().into_owned())
    }
    #[cfg(not(all(target_os = "linux", target_env = "gnu")))]
    {
        None
    }
}
pub fn at_least(actual: &str, minimum: (u32, u32)) -> bool {
    let mut s = actual.split('.').filter_map(|s| s.parse::<u32>().ok());
    (s.next().unwrap_or(0), s.next().unwrap_or(0)) >= minimum
}
pub fn artifact_preflight(component: runtime::Component) -> Result<()> {
    ensure!(
        cfg!(all(target_os = "linux", target_arch = "x86_64")),
        "Automatic binaries support Linux x86_64; use a compatible existing runtime on this machine"
    );
    let minimum = match component {
        runtime::Component::Engine => (2, 28),
        runtime::Component::Inference => (2, 34),
    };
    let libc = glibc().context(
        "These managed GNU binaries need glibc; use a compatible manually installed runtime",
    )?;
    ensure!(
        at_least(&libc, minimum),
        "{} needs glibc {}.{} or newer, detected {}. No download started. Build this component from source or connect to a server on another machine.",
        component.description(),
        minimum.0,
        minimum.1,
        libc
    );
    Ok(())
}
pub async fn inspect() -> Report {
    let h = runtime::hardware().await;
    let cpu = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let flags = cpu.lines().find(|l| l.starts_with("flags")).unwrap_or("");
    let cpu_flags = ["sse2", "sse4_2", "avx", "avx2", "avx512f"]
        .into_iter()
        .filter(|flag| flags.split_whitespace().any(|f| f == *flag))
        .map(str::to_owned)
        .collect();
    let budget = h.available_ram.saturating_sub(2 * 1024 * 1024 * 1024) / 2;
    let profiles=vec![
        Choice{name:"Low memory".into(),context:4096,model_hint:"1.7–3B Q4 GGUF; verify an uncensored derivative".into(),estimated_weight_budget:budget.min(2*1024*1024*1024),explanation:"One loaded model, small context, leave RAM for the desktop and tools".into()},
        Choice{name:"Balanced".into(),context:8192,model_hint:"3–4B Q4 GGUF; first evaluation target for 16 GB RAM".into(),estimated_weight_budget:budget,explanation:"A starting estimate. Weight size excludes KV cache, runtime and process memory".into()},
        Choice{name:"More context".into(),context:16384,model_hint:"Use a model whose native context supports this size".into(),estimated_weight_budget:budget.saturating_sub(1024*1024*1024),explanation:"Run the capability check, then measure memory and latency. Reduce context on allocation failure".into()},
    ];
    let mut command = tokio::process::Command::new("nvidia-smi");
    command
        .args([
            "--query-gpu=driver_version,memory.free,compute_cap",
            "--format=csv,noheader,nounits",
        ])
        .kill_on_drop(true);
    let driver = match tokio::time::timeout(Duration::from_secs(2), command.output()).await {
        Ok(Ok(r)) if r.status.success() => {
            crate::display_text(String::from_utf8_lossy(&r.stdout).trim())
        }
        _ => "Driver/free VRAM/compute capability unavailable".into(),
    };
    Report{os:std::env::consts::OS.into(),architecture:std::env::consts::ARCH.into(),libc:glibc(),ram:h.ram,available_ram:h.available_ram,cpu_threads:h.threads,cpu_flags,gpu:h.gpu,driver,isolation:crate::sandbox::probe().await,profiles,notes:vec!["RAM and discrete VRAM are separate budgets. Estimates do not guarantee fit or tool quality.".into(),"GTX 1070 is Pascal sm_61. Use a compatible CUDA 12 build or CPU runtime; CUDA 13-only binaries do not cover it.".into(),"Spark-X2.5 and MiMo are discoverable through arbitrary Hub search/compatible endpoints; publisher uncensored labels require separate verification.".into(),"The managed download defaults to CPU. Select a compatible GPU runtime and configure layers in Settings, or use an external Ollama/llama.cpp/LM Studio/ORA endpoint.".into()]}
}
#[derive(Debug, Clone, Serialize)]
pub struct Capability {
    pub configuration_identity: Value,
    pub id: String,
    pub model: String,
    pub endpoint: String,
    pub provider: Provider,
    pub context: u32,
    pub artifact: Option<Value>,
    pub native_tool_call: bool,
    pub tool_result_used: bool,
    pub elapsed_ms: u128,
    pub observations: Vec<Value>,
    pub error: Option<String>,
}
/// Harmless two-step arithmetic/nonce tool roundtrip. Never chooses a different model.
pub async fn evaluate(
    root: &Path,
    preferences: &Preferences,
    profile: &Profile,
    cancel: Arc<AtomicBool>,
) -> Result<Capability> {
    ensure!(
        profile.uncensored,
        "Live evaluation requires an explicitly selected uncensored/abliterated model. Mark its publisher/user claim first; Alt will not substitute a model"
    );
    let mut effective = profile.clone();
    let mut local = None;
    if profile.local_model.is_some() {
        let runtime =
            runtime::LocalRuntime::start(root, preferences, profile, cancel.clone(), |_| {})
                .await?;
        effective.endpoint = runtime.endpoint.clone();
        local = Some(runtime);
    }
    let id = uuid::Uuid::new_v4().to_string();
    let marker = format!("ALT_PROBE_{}", &id[..8]);
    let started = Instant::now();
    let data = root.to_path_buf();
    let prefs = preferences.clone();
    let selected = profile.clone();
    let identity = tokio::task::spawn_blocking(move || {
        crate::capability::identity(&data, &prefs, &selected, None)
    })
    .await??;
    let mut report = Capability {
        configuration_identity: identity,
        id,
        model: effective.model.clone(),
        endpoint: effective.endpoint.clone(),
        provider: effective.provider,
        context: effective.context_tokens,
        artifact: profile
            .local_model
            .as_ref()
            .and_then(|id| {
                models::library(root)
                    .ok()?
                    .into_iter()
                    .find(|a| &a.id == id)
            })
            .and_then(|a| serde_json::to_value(a).ok()),
        native_tool_call: false,
        tool_result_used: false,
        elapsed_ms: 0,
        observations: vec![],
        error: None,
    };
    let operation = async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()?;
        let url = format!(
            "{}/{}",
            effective.endpoint.trim_end_matches('/'),
            if effective.provider == Provider::Ollama {
                "api/chat"
            } else {
                "chat/completions"
            }
        );
        let tools = json!([{"type":"function","function":{"name":"alt_probe","description":"Return the sum of two numbers and a secret receipt. Call this to obtain the receipt.","parameters":{"type":"object","properties":{"left":{"type":"integer"},"right":{"type":"integer"}},"required":["left","right"],"additionalProperties":false}}}]);
        let mut messages = vec![
            json!({"role":"user","content":"Call alt_probe with left=19 and right=23. Then report the exact sum and receipt returned by the tool. Do not guess the receipt."}),
        ];
        for step in 0..2 {
            ensure!(
                !cancel.load(std::sync::atomic::Ordering::Relaxed),
                "Capability check cancelled"
            );
            let mut body =
                json!({"model":effective.model,"messages":messages,"tools":tools,"stream":false});
            if effective.provider == Provider::Ollama {
                body["options"] =
                    json!({"num_ctx":effective.context_tokens,"num_predict":256,"temperature":0});
            } else {
                body["max_tokens"] = json!(256);
                body["temperature"] = json!(0);
            }
            let mut request = client.post(&url).json(&body);
            if let Some(key) = effective.key()? {
                request = request.bearer_auth(key);
            }
            let response = request.send().await?.error_for_status()?;
            ensure!(
                response.content_length().unwrap_or(0) <= 1024 * 1024,
                "Probe response too large"
            );
            let mut stream = response.bytes_stream();
            let mut bytes = Vec::new();
            use futures_util::StreamExt;
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                ensure!(
                    bytes.len() + chunk.len() <= 1024 * 1024,
                    "Probe response exceeds 1 MiB"
                );
                bytes.extend_from_slice(&chunk);
            }
            let value: Value = serde_json::from_slice(&bytes)?;
            let message = if effective.provider == Provider::Ollama {
                value["message"].clone()
            } else {
                value["choices"][0]["message"].clone()
            };
            report.observations.push(message.clone());
            if step == 0 {
                let calls = message["tool_calls"]
                    .as_array()
                    .context("Model did not produce a native tool call")?;
                ensure!(calls.len() == 1, "Expected exactly one tool call");
                let call = &calls[0];
                let args = match &call["function"]["arguments"] {
                    Value::String(s) => serde_json::from_str::<Value>(s)?,
                    other => other.clone(),
                };
                ensure!(
                    call["function"]["name"] == "alt_probe"
                        && args == json!({"left":19,"right":23}),
                    "Tool name or arguments were incorrect"
                );
                report.native_tool_call = true;
                messages.push(message.clone());
                messages.push(json!({"role":"tool","name":"alt_probe","tool_call_id":call["id"].as_str().unwrap_or("alt-probe"),"content":json!({"sum":42,"receipt":marker}).to_string()}));
            } else {
                let content = message["content"].as_str().unwrap_or("");
                report.tool_result_used = content.contains("42") && content.contains(&marker);
                ensure!(
                    report.tool_result_used,
                    "Model did not use the actual tool result/receipt"
                );
            }
        }
        Ok(())
    };
    let outcome: Result<()> = tokio::select! { result=operation=>result, _=models::cancelled(&cancel)=>Err(anyhow::anyhow!("Capability check cancelled")) };
    if let Err(e) = outcome {
        report.error = Some(format!("{e:#}"));
    }
    report.elapsed_ms = started.elapsed().as_millis();
    if let Some(runtime) = &mut local {
        runtime.stop().await;
    }
    let output = root.join("evaluations");
    crate::config::private_dir(&output)?;
    crate::config::atomic_write(
        &output.join(format!("{}.json", report.id)),
        &serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}
