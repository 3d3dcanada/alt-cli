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
    pub vram_total: Option<u64>,
    pub vram_free: Option<u64>,
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
        Choice{name:"Balanced".into(),context:8192,model_hint:"Use selected-model Fit for your 7B/9B Q4 or smaller artifact".into(),estimated_weight_budget:budget,explanation:"Generic RAM guidance only. Selected-model Fit uses architecture, KV cache and separate GPU budgets; calibration measures the result".into()},
        Choice{name:"More context".into(),context:16384,model_hint:"Use a model whose native context supports this size".into(),estimated_weight_budget:budget.saturating_sub(1024*1024*1024),explanation:"Run the capability check, then measure memory and latency. Reduce context on allocation failure".into()},
    ];
    let mut command = tokio::process::Command::new("nvidia-smi");
    command.args([
        "--query-gpu=driver_version,memory.free,compute_cap",
        "--format=csv,noheader,nounits",
    ]);
    let driver =
        match crate::process::bounded_output(&mut command, Duration::from_secs(2), 64 * 1024).await
        {
            Ok(r) if r.status.success() => {
                crate::display_text(String::from_utf8_lossy(&r.stdout).trim())
            }
            _ => "Driver/free VRAM/compute capability unavailable".into(),
        };
    Report{os:std::env::consts::OS.into(),architecture:std::env::consts::ARCH.into(),libc:glibc(),ram:h.ram,available_ram:h.available_ram,cpu_threads:h.threads,cpu_flags,gpu:h.gpu,driver,vram_total:h.vram_total,vram_free:h.vram_free,isolation:crate::sandbox::probe().await,profiles,notes:vec!["RAM and discrete VRAM are separate budgets. Estimates do not guarantee fit or tool quality.".into(),"GTX 1070 is Pascal sm_61. Use a compatible CUDA 12 build or CPU runtime; CUDA 13-only binaries do not cover it.".into(),"Spark-X2.5 and MiMo are discoverable through arbitrary Hub search/compatible endpoints; publisher uncensored labels require separate verification.".into(),"The managed download defaults to CPU. Select a compatible GPU runtime and configure layers in Settings, or use an external Ollama/llama.cpp/LM Studio/ORA endpoint.".into()]}
}

/// Model-aware proposals; this function never loads weights or changes settings.
pub async fn model_fit(root: &Path, preferences: &Preferences, profile: &Profile) -> Result<Value> {
    let detected = runtime::hardware().await;
    let Some(id) = profile.local_model.as_ref() else {
        return Ok(
            json!({"model":profile.model,"status":"unmeasured","reason":"External endpoint owns model allocation. Inspect its observed capabilities and run explicit calibration.","changed_settings":false}),
        );
    };
    let artifact = models::artifact(root, id)?;
    let metadata = models::inspect_gguf(&artifact.path).await?;
    let mut proposals = Vec::new();
    for context in [4096u32, 8192] {
        if metadata
            .native_context
            .is_some_and(|n| n < u64::from(context))
        {
            continue;
        }
        let proposed_gpu = detected
            .vram_free
            .zip(metadata.blocks)
            .zip(kv_bytes(&metadata, context, &preferences.runtime))
            .and_then(|((free, blocks), kv)| {
                let room = free.saturating_sub(768 * 1024 * 1024);
                let total = artifact.bytes.checked_add(kv)?;
                let blocks = blocks.checked_add(1)?;
                if total == 0 {
                    return None;
                }
                let layers = (u128::from(room) * u128::from(blocks) / u128::from(total))
                    .min(u128::from(blocks));
                i32::try_from(layers).ok().filter(|n| *n > 0)
            });
        for layers in [Some(0), Some(preferences.runtime.gpu_layers), proposed_gpu]
            .into_iter()
            .flatten()
        {
            if proposals
                .iter()
                .any(|p: &Value| p["context"] == context && p["gpu_layers"] == layers)
            {
                continue;
            }
            proposals.push(fit_estimate(
                &metadata,
                artifact.bytes,
                context,
                layers,
                &preferences.runtime,
                &detected,
            ));
        }
    }
    Ok(
        json!({"model":profile.model,"artifact_id":artifact.id,"metadata":metadata,"available_ram":detected.available_ram,"first_device_free_vram":detected.vram_free,"proposals":proposals,"changed_settings":false,"qualification":"Estimates only. Select settings explicitly, then measure loading, first output, memory and independent task correctness. Model bytes exclude runtime buffers; KV estimates assume standard full attention and may overestimate hybrid attention. Unsupported architectures remain unknown."}),
    )
}
pub fn kv_bytes(
    m: &models::gguf::Metadata,
    context: u32,
    settings: &runtime::Settings,
) -> Option<u64> {
    let heads = m.attention_heads.filter(|n| *n > 0)?;
    let head = m.embedding?.checked_div(heads)?;
    let keys = m.key_length.unwrap_or(head);
    let values = m.value_length.unwrap_or(head);
    let elements = m
        .blocks?
        .checked_mul(m.kv_heads.unwrap_or(heads))?
        .checked_mul(u64::from(context))?;
    let storage = |n: u64, kind: &str| match kind {
        "f16" => n.checked_mul(2),
        "q8_0" => n.div_ceil(32).checked_mul(34),
        "q4_0" => n.div_ceil(32).checked_mul(18),
        _ => None,
    };
    storage(elements.checked_mul(keys)?, &settings.cache_k)?
        .checked_add(storage(elements.checked_mul(values)?, &settings.cache_v)?)
}
fn fit_estimate(
    m: &models::gguf::Metadata,
    weights: u64,
    context: u32,
    layers: i32,
    settings: &runtime::Settings,
    hardware: &runtime::Hardware,
) -> Value {
    let kv = kv_bytes(m, context, settings);
    let total = m.blocks.and_then(|n| n.checked_add(1));
    let fraction = total.filter(|n| *n > 0).map(|n| {
        if layers < 0 {
            1.0
        } else {
            (layers as f64 / n as f64).clamp(0.0, 1.0)
        }
    });
    let gpu_weights = if layers == 0 {
        Some(0)
    } else {
        fraction.map(|f| (weights as f64 * f).ceil() as u64)
    };
    let gpu_kv = if layers == 0 {
        Some(0)
    } else {
        kv.zip(fraction).map(|(n, f)| (n as f64 * f).ceil() as u64)
    };
    let gpu = gpu_weights.zip(gpu_kv).and_then(|(w, k)| {
        w.checked_add(k)?
            .checked_add(if layers == 0 { 0 } else { 512 * 1024 * 1024 })
    });
    let ram = kv.zip(gpu_weights).zip(gpu_kv).and_then(|((k, wg), kg)| {
        weights
            .saturating_sub(wg)
            .checked_add(k.saturating_sub(kg))?
            .checked_add(1024 * 1024 * 1024)
    });
    json!({"context":context,"gpu_layers":layers,"batch":settings.batch,"cache_k":settings.cache_k,"cache_v":settings.cache_v,"estimated_kv_bytes":kv,"estimated_ram_bytes":ram,"estimated_vram_bytes":gpu,"ram_estimate_fits":ram.map(|n|n<=hardware.available_ram),"vram_estimate_fits":if layers==0{Some(true)}else{gpu.zip(hardware.vram_free).map(|(n,free)|n<=free)},"native_context_declared":m.native_context,"scope":"Rough weight/layer allocation plus KV and buffer reserve; not measured fit, kernel support or actual offload"})
}

/// Observe server-advertised capabilities without generating or switching models.
pub async fn provider_capabilities(profile: &Profile) -> Result<Value> {
    let mut report = json!({"provider":profile.provider,"model":profile.model,"endpoint":profile.endpoint,"observed_native_context":null,"tokenization":"unknown","tools":"unknown until actual native tool roundtrip","weights_verified_by_alt":profile.local_model.is_some()});
    let client = runtime::http_client(profile)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()?;
    let (url, body) = if profile.provider == Provider::Ollama {
        (
            format!("{}/api/show", profile.endpoint.trim_end_matches('/')),
            Some(json!({"model":profile.model})),
        )
    } else {
        (
            format!(
                "{}/props",
                profile
                    .endpoint
                    .trim_end_matches('/')
                    .trim_end_matches("/v1")
            ),
            None,
        )
    };
    let mut request = if let Some(body) = body {
        client.post(url).json(&body)
    } else {
        client.get(url)
    };
    if let Some(key) = profile.key()? {
        request = request.bearer_auth(key);
    }
    let result = async {
        let mut response = request.send().await?.error_for_status()?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            ensure!(
                bytes.len() + chunk.len() <= 2 * 1024 * 1024,
                "Provider capability response exceeds 2 MiB"
            );
            bytes.extend_from_slice(&chunk);
        }
        Ok::<Value, anyhow::Error>(serde_json::from_slice(&bytes)?)
    }
    .await;
    match result {
        Ok(value) => {
            if profile.provider == Provider::Ollama {
                report["declared_capabilities"] = value["capabilities"].clone();
                report["declared_model_context"] = value["model_info"]
                    .as_object()
                    .and_then(|m| m.iter().find(|(k, _)| k.ends_with(".context_length")))
                    .map(|(_, v)| v.clone())
                    .unwrap_or(Value::Null);
                report["context_scope"] = json!(
                    "Model metadata maximum; active native context is not established by /api/show"
                );
            } else {
                report["observed_native_context"] =
                    value["default_generation_settings"]["n_ctx"].clone();
                report["template_present"] = json!(value["chat_template"].is_string());
                if profile.local_model.is_some() {
                    report["tokenization"] =
                        json!("Owned runtime apply-template/tokenize measured on actual requests");
                }
            }
        }
        Err(e) => report["observation_error"] = json!(format!("{e:#}")),
    }
    Ok(report)
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
        let client = runtime::http_client(&effective)
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
