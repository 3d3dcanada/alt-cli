//! Opt-in, owned local inference and pinned Linux runtime installers.
use crate::{
    config::{Preferences, Profile, atomic_write, private_dir},
    models::{self, Cancel, Progress},
};
use anyhow::{Context, Result, bail, ensure};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::atomic::Ordering,
    time::Duration,
};
use tokio::process::{Child, Command};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub gpu_layers: i32,
    pub threads: usize,
    pub batch: u32,
    pub cache_k: String,
    pub cache_v: String,
    pub flash_attention: bool,
    pub thinking: Option<bool>,
    /// A02 named llama-server settings. Unset means the runtime's default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mmproj: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_cpu_moe: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_tensor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tensor_split: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_gpu: Option<u32>,
    /// none, linear or yarn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rope_scaling: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rope_scale: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yarn_orig_ctx: Option<u32>,
    pub mlock: bool,
    pub no_mmap: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_template_file: Option<PathBuf>,
    /// Speculative decoding with a smaller draft model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_model: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_max: Option<u32>,
    /// Further llama-server arguments, appended last. Alt-owned flags are refused.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extra_args: Vec<String>,
}

/// llama-server flags Alt owns: binding, credentials, the model and its alias.
pub const RESERVED_RUNTIME_ARGS: [&str; 10] = [
    "--host",
    "--port",
    "--api-key",
    "--api-key-file",
    "--alias",
    "-a",
    "--model",
    "-m",
    "--ctx-size",
    "-c",
];
impl Default for Settings {
    fn default() -> Self {
        Self {
            gpu_layers: 0,
            threads: 0,
            batch: 128,
            cache_k: "f16".into(),
            cache_v: "f16".into(),
            flash_attention: false,
            thinking: None,
            mmproj: None,
            n_cpu_moe: None,
            override_tensor: None,
            tensor_split: None,
            main_gpu: None,
            rope_scaling: None,
            rope_scale: None,
            yarn_orig_ctx: None,
            mlock: false,
            no_mmap: false,
            chat_template_file: None,
            draft_model: None,
            draft_max: None,
            extra_args: Vec::new(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (-1..=200).contains(&self.gpu_layers)
                && self.threads <= 1024
                && (16..=8192).contains(&self.batch),
            "Runtime settings: GPU layers -1–200, threads 0–1024, batch 16–8192"
        );
        ensure!(
            ["f16", "q8_0", "q4_0"].contains(&self.cache_k.as_str())
                && ["f16", "q8_0", "q4_0"].contains(&self.cache_v.as_str()),
            "KV cache type must be f16, q8_0 or q4_0"
        );
        ensure!(
            self.cache_v == "f16" || self.flash_attention,
            "Quantized V cache requires compatible flash attention; use f16 on unsupported hardware"
        );
        ensure!(
            self.rope_scaling
                .as_deref()
                .is_none_or(|r| ["none", "linear", "yarn"].contains(&r)),
            "RoPE scaling must be none, linear or yarn"
        );
        ensure!(
            self.rope_scale
                .is_none_or(|n| n.is_finite() && n > 0.0 && n <= 64.0),
            "RoPE scale must be above 0 and at most 64"
        );
        ensure!(
            self.tensor_split.as_deref().is_none_or(|t| {
                !t.is_empty()
                    && t.split(',')
                        .all(|n| n.trim().parse::<f64>().is_ok_and(|n| n >= 0.0))
            }),
            "Tensor split is a comma-separated list of proportions, such as 3,1"
        );
        ensure!(
            self.override_tensor
                .as_deref()
                .is_none_or(|t| !t.trim().is_empty() && !t.chars().any(char::is_control)),
            "Tensor override must be a non-empty pattern=buffer list"
        );
        ensure!(
            self.n_cpu_moe.is_none_or(|n| n <= 1024)
                && self.main_gpu.is_none_or(|n| n < 64)
                && self.draft_max.is_none_or(|n| (1..=64).contains(&n))
                && self.yarn_orig_ctx.is_none_or(|n| n >= 256),
            "MoE CPU layers 0–1024, main GPU 0–63, draft tokens 1–64, YaRN original context at least 256"
        );
        ensure!(
            self.draft_max.is_none() || self.draft_model.is_some(),
            "Choose a draft model before setting draft tokens"
        );
        ensure!(
            self.extra_args.len() <= 64,
            "Use at most 64 extra runtime arguments"
        );
        for arg in &self.extra_args {
            ensure!(
                !arg.chars().any(char::is_control),
                "Extra runtime arguments may not contain control characters"
            );
            let flag = arg.split('=').next().unwrap_or("");
            ensure!(
                !RESERVED_RUNTIME_ARGS.contains(&flag),
                "Alt owns `{flag}` (binding, credentials, model, alias and context); remove it from extra runtime arguments"
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Hardware {
    pub ram: u64,
    pub available_ram: u64,
    pub threads: usize,
    pub gpu: String,
}

// Container usage includes reclaimable disk cache. Match the conservative
// working-set convention: subtract inactive file pages, not anonymous memory,
// tmpfs/shmem or the active file working set. Host MemAvailable remains a ceiling.
fn cgroup_available(host_available: u64, limit: u64, used: u64, statistics: &str) -> u64 {
    let inactive = statistics
        .lines()
        .find_map(|line| {
            let mut fields = line.split_whitespace();
            (fields.next() == Some("inactive_file"))
                .then(|| fields.next()?.parse::<u64>().ok())
                .flatten()
        })
        .unwrap_or(0)
        .min(used);
    host_available.min(limit.saturating_sub(used.saturating_sub(inactive)))
}

#[cfg(test)]
mod selection_tests {
    use super::*;
    #[test]
    fn a_selected_but_missing_executable_never_falls_back() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let managed = root.path().join("tools/llama-b11429/llama-server");
        std::fs::create_dir_all(managed.parent().unwrap()).unwrap();
        std::fs::write(&managed, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&managed, std::fs::Permissions::from_mode(0o755)).unwrap();
        let engine = root.path().join("tools/goose");
        std::fs::write(&engine, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&engine, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut preferences = Preferences::default();
        assert_eq!(
            find_runtime(root.path(), &preferences),
            Some(managed.clone())
        );
        assert_eq!(
            find_engine(root.path(), Path::new("goose"), &preferences),
            Some(engine.clone())
        );
        preferences.runtime_path = Some(root.path().join("deleted/llama-server"));
        preferences.engine_path = Some(root.path().join("deleted/goose"));
        assert_eq!(find_runtime(root.path(), &preferences), None);
        assert_eq!(
            find_engine(root.path(), Path::new("goose"), &preferences),
            None
        );
        preferences.runtime_path = Some(managed.clone());
        assert_eq!(find_runtime(root.path(), &preferences), Some(managed));
    }
}

#[cfg(test)]
mod gpu_tests {
    use super::*;
    #[test]
    fn nvidia_smi_rows_and_cuda12_fit() {
        let gpus = parse_gpus(
            "NVIDIA GeForce GTX 1070, 8192, 6.1, 550.54.14\nTesla K80, 11441, 3.7, 470.82.01\n",
        );
        assert_eq!(gpus.len(), 2);
        assert_eq!(gpus[0].compute, Some((6, 1)));
        assert_eq!(gpus[0].memory_mib, 8192);
        assert!(cuda12_fit(&gpus[..1]).unwrap().contains("GTX 1070"));
        assert!(cuda12_fit(&gpus[1..]).unwrap_err().contains("6.1"));
        let old_driver = parse_gpus("NVIDIA GeForce GTX 1070, 8192, 6.1, 470.82.01");
        assert!(cuda12_fit(&old_driver).unwrap_err().contains("525.60"));
        assert!(cuda12_fit(&[]).unwrap_err().contains("No NVIDIA GPU"));
        assert!(parse_gpus("").is_empty());
        assert!(CUDA12.is_none() || CUDA12.is_some_and(|p| p.sha256.len() == 64));
    }
}

#[cfg(test)]
mod memory_tests {
    use super::cgroup_available;

    #[test]
    fn container_cache_does_not_hide_available_ram_or_free_anonymous_memory() {
        let stats = "anon 20\nshmem 5\nactive_file 12\ninactive_file 6\n";
        assert_eq!(cgroup_available(16, 32, 31, stats), 7);
        assert_eq!(cgroup_available(2, 32, 31, stats), 2);
        assert_eq!(cgroup_available(16, 32, 33, stats), 5);
        assert_eq!(cgroup_available(16, 32, 31, "anon 20\nshmem 5\n"), 1);
        assert_eq!(cgroup_available(16, 32, 31, "inactive_file invalid"), 1);
        assert_eq!(cgroup_available(16, 32, 40, "inactive_file 0"), 0);
    }
}

pub async fn hardware() -> Hardware {
    let memory = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let field = |name: &str| {
        memory
            .lines()
            .find(|line| line.starts_with(name))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|n| n.parse::<u64>().ok())
            .unwrap_or(0)
            * 1024
    };
    let mut data = Hardware {
        ram: field("MemTotal:"),
        available_ram: field("MemAvailable:"),
        threads: std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1),
        gpu: "No NVIDIA GPU detected; CPU mode is available".into(),
    };
    if let Ok(limit) = std::fs::read_to_string("/sys/fs/cgroup/memory.max")
        && let Ok(limit) = limit.trim().parse::<u64>()
    {
        let used = std::fs::read_to_string("/sys/fs/cgroup/memory.current")
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);
        data.ram = data.ram.min(limit);
        let statistics = std::fs::read_to_string("/sys/fs/cgroup/memory.stat").unwrap_or_default();
        data.available_ram = cgroup_available(data.available_ram, limit, used, &statistics);
    }
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        Command::new("nvidia-smi")
            .args([
                "--query-gpu=name,memory.total",
                "--format=csv,noheader,nounits",
            ])
            .kill_on_drop(true)
            .output(),
    )
    .await;
    if let Ok(Ok(result)) = result
        && result.status.success()
    {
        data.gpu = crate::display_text(String::from_utf8_lossy(&result.stdout).trim());
    }
    data
}

pub fn find_engine(root: &Path, requested: &Path, preferences: &Preferences) -> Option<PathBuf> {
    if requested != Path::new("goose") {
        return find_executable(requested);
    }
    // B06: an explicitly selected executable is authoritative; no fallback.
    if let Some(selected) = preferences.engine_path.as_deref() {
        return find_executable(selected);
    }
    find_executable(&root.join("tools/goose"))
        .or_else(|| find_executable(Path::new("goose")))
        .or_else(|| find_executable(Path::new("/workspace/.alt-tools/goose")))
}

pub fn find_runtime(root: &Path, preferences: &Preferences) -> Option<PathBuf> {
    // B06: an explicitly selected executable is authoritative; no fallback.
    if let Some(selected) = preferences.runtime_path.as_deref() {
        return find_executable(selected);
    }
    find_executable(&root.join("tools/llama-b11429/llama-server"))
        .or_else(|| find_executable(Path::new("llama-server")))
        .or_else(|| {
            find_executable(Path::new(
                "/workspace/.alt-tools/llama-b11429/llama-b11429/llama-server",
            ))
        })
}

fn find_executable(path: &Path) -> Option<PathBuf> {
    let valid = |path: &Path| {
        if !path.is_file() {
            return false;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            path.metadata()
                .is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
        }
        #[cfg(not(unix))]
        {
            true
        }
    };
    if path.components().count() > 1 {
        return valid(path).then(|| path.to_path_buf());
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(path))
            .find(|p| valid(p))
    })
}

#[derive(Debug, Clone, Copy)]
pub enum Component {
    Engine,
    Inference,
    /// A08: optional llama.cpp CUDA 12 build for NVIDIA GPUs (Pascal sm_61+).
    CudaInference,
}

/// A pinned, checksummed runtime archive with one top-level directory that
/// contains `llama-server` (and any shared libraries beside it).
#[derive(Debug, Clone, Copy)]
pub struct Pin {
    pub label: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub size: u64,
    pub directory: &'static str,
}

/// A08: the owner fills this from a CUDA 12 build verified on the reference PC
/// (`scripts/pin_cuda_runtime.py` prints the exact values). Until then automatic
/// CUDA installation is unavailable; CPU stays the default and Alt explains how
/// to build or select a GPU runtime instead. Never auto-switched.
pub const CUDA12: Option<Pin> = None;

/// One NVIDIA GPU as reported by `nvidia-smi`.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Gpu {
    pub name: String,
    pub memory_mib: u64,
    pub compute: Option<(u32, u32)>,
    pub driver: Option<String>,
}

/// Parse `nvidia-smi --query-gpu=name,memory.total,compute_cap,driver_version
/// --format=csv,noheader,nounits`. Unknown fields stay unknown.
pub fn parse_gpus(text: &str) -> Vec<Gpu> {
    text.lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(',').map(str::trim).collect();
            let name = fields.first().filter(|n| !n.is_empty())?.to_string();
            let compute = fields.get(2).and_then(|c| {
                let (major, minor) = c.split_once('.')?;
                Some((major.parse().ok()?, minor.parse().ok()?))
            });
            Some(Gpu {
                name: crate::display_text(&name),
                memory_mib: fields.get(1).and_then(|m| m.parse().ok()).unwrap_or(0),
                compute,
                driver: fields
                    .get(3)
                    .filter(|d| !d.is_empty())
                    .map(|d| d.to_string()),
            })
        })
        .collect()
}

pub async fn detect_gpus() -> Vec<Gpu> {
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        Command::new("nvidia-smi")
            .args([
                "--query-gpu=name,memory.total,compute_cap,driver_version",
                "--format=csv,noheader,nounits",
            ])
            .kill_on_drop(true)
            .output(),
    )
    .await;
    match result {
        Ok(Ok(output)) if output.status.success() => {
            parse_gpus(&String::from_utf8_lossy(&output.stdout))
        }
        _ => Vec::new(),
    }
}

/// Whether a CUDA 12 runtime can serve these GPUs: compute capability 6.1+
/// (Pascal and newer) and a driver new enough for CUDA 12 (525.60+). A labelled
/// reason either way; nothing is installed or selected here.
pub fn cuda12_fit(gpus: &[Gpu]) -> std::result::Result<String, String> {
    let gpu = gpus.iter().max_by_key(|g| g.memory_mib).ok_or_else(|| {
        "No NVIDIA GPU was detected (nvidia-smi unavailable). CPU mode stays available.".to_string()
    })?;
    let compute = gpu.compute.ok_or_else(|| {
        format!(
            "{} reports no compute capability; check the NVIDIA driver",
            gpu.name
        )
    })?;
    if compute < (6, 1) {
        return Err(format!(
            "{} has compute capability {}.{}; CUDA 12 builds here need 6.1 (Pascal) or newer",
            gpu.name, compute.0, compute.1
        ));
    }
    let driver_ok = gpu
        .driver
        .as_deref()
        .and_then(|d| {
            let mut parts = d.split('.').filter_map(|p| p.parse::<u32>().ok());
            Some((parts.next()?, parts.next().unwrap_or(0)))
        })
        .is_some_and(|version| version >= (525, 60));
    if !driver_ok {
        return Err(format!(
            "{} needs NVIDIA driver 525.60 or newer for CUDA 12 (found {})",
            gpu.name,
            gpu.driver.as_deref().unwrap_or("unknown")
        ));
    }
    Ok(format!(
        "{} · {} MiB · compute {}.{} · driver {}: a CUDA 12 runtime can use this GPU",
        gpu.name,
        gpu.memory_mib,
        compute.0,
        compute.1,
        gpu.driver.as_deref().unwrap_or("?")
    ))
}

impl Component {
    pub fn description(self) -> &'static str {
        match self {
            Self::Engine => "Goose 1.53.0 agent engine",
            Self::Inference => "llama.cpp b11429 CPU runtime",
            Self::CudaInference => {
                "llama.cpp CUDA 12 runtime for NVIDIA GPUs (Pascal sm_61 and newer)"
            }
        }
    }
    pub fn size(self) -> u64 {
        match self {
            Self::Engine => 94_629_008,
            Self::Inference => 17_693_462,
            Self::CudaInference => CUDA12.map_or(0, |p| p.size),
        }
    }
    fn release(self) -> (&'static str, &'static str) {
        match self {
            Self::Engine => (
                "https://github.com/aaif-goose/goose/releases/download/v1.53.0/goose-x86_64-unknown-linux-gnu.tar.gz",
                "deb2191a6b75acc0a20232fc5c52655ea2f9cc8fa2f5dffc8622e8d378a915dc",
            ),
            Self::Inference => (
                "https://github.com/ggml-org/llama.cpp/releases/download/b11429/llama-b11429-bin-ubuntu-x64.tar.gz",
                "f6d25dde8f51133143d1453da4fd5f73b145127177612a283bf7995957af3392",
            ),
            Self::CudaInference => CUDA12.map_or(("", ""), |p| (p.url, p.sha256)),
        }
    }
}

pub async fn install(
    root: &Path,
    component: Component,
    cancel: Cancel,
    mut progress: impl FnMut(Progress),
) -> Result<PathBuf> {
    if let Component::CudaInference = component {
        let pin = CUDA12.context(
            "No CUDA 12 runtime is pinned in this Alt version yet. Build one with scripts/build-pascal-runtime.sh and select its llama-server in Settings, or keep CPU mode",
        )?;
        cuda12_fit(&detect_gpus().await).map_err(anyhow::Error::msg)?;
        crate::hardware::artifact_preflight(component)?;
        return install_pin(root, &pin, cancel, progress).await;
    }
    ensure!(
        cfg!(all(target_os = "linux", target_arch = "x86_64")),
        "Automatic installation currently supports Linux x86_64. Set an existing executable in Settings on this platform"
    );
    crate::hardware::artifact_preflight(component)?;
    let tools = root.join("tools");
    private_dir(&tools)?;
    let (url, hash) = component.release();
    let archive = tools.join(format!("{hash}.tar.gz"));
    models::download_verified(
        url,
        &archive,
        component.size(),
        hash,
        cancel.clone(),
        &mut progress,
    )
    .await?;
    ensure!(!cancel.load(Ordering::Relaxed), "Installation cancelled");
    progress(Progress {
        received: component.size(),
        total: component.size(),
        stage: "Installing verified runtime".into(),
    });
    let installed=tokio::task::spawn_blocking(move || -> Result<PathBuf> {
        let temp=tempfile::tempdir_in(&tools)?;
        let gzip=flate2::read::GzDecoder::new(std::fs::File::open(&archive)?);
        let mut tar=tar::Archive::new(gzip);
        tar.unpack(temp.path()).context("Could not unpack the verified release")?;
        let destination=match component {
            Component::Engine=>{
                let dest=tools.join("goose");
                std::fs::rename(temp.path().join("goose"),&dest)?;
                dest
            },
            Component::CudaInference=>bail!("CUDA runtimes install through their pin"),
            Component::Inference=>{
                let dest=tools.join("llama-b11429");
                let previous=temp.path().join("previous");
                if dest.exists() { std::fs::rename(&dest,&previous)?; }
                if let Err(error)=std::fs::rename(temp.path().join("llama-b11429"),&dest) {
                    if previous.exists() { std::fs::rename(&previous,&dest)?; }
                    return Err(error.into());
                }
                dest.join("llama-server")
            },
        };
        atomic_write(&tools.join("PROVENANCE.txt"),b"Goose 1.53.0: Apache-2.0, https://github.com/aaif-goose/goose\nllama.cpp b11429: MIT, https://github.com/ggml-org/llama.cpp\nRelease archives are pinned by SHA256 in Alt's runtime.rs.\n")?;
        Ok(destination)
    }).await??;
    Ok(installed)
}

/// The exact llama-server argv Alt launches, in order. Pure, for review and tests.
pub fn server_args(
    settings: &Settings,
    profile: &Profile,
    model: &Path,
    port: u16,
    threads: usize,
    key_file: &Path,
) -> Vec<std::ffi::OsString> {
    let mut args: Vec<std::ffi::OsString> = vec!["--model".into(), model.into()];
    for (flag, value) in [
        ("--ctx-size", profile.context_tokens.to_string()),
        ("--host", "127.0.0.1".into()),
        ("--port", port.to_string()),
        ("--threads", threads.to_string()),
        ("--parallel", "1".into()),
        ("--n-gpu-layers", settings.gpu_layers.to_string()),
        ("--batch-size", settings.batch.to_string()),
        ("--ubatch-size", settings.batch.min(512).to_string()),
        ("--cache-type-k", settings.cache_k.clone()),
        ("--cache-type-v", settings.cache_v.clone()),
        (
            "--flash-attn",
            if settings.flash_attention {
                "on"
            } else {
                "off"
            }
            .into(),
        ),
    ] {
        args.push(flag.into());
        args.push(value.into());
    }
    args.push("--jinja".into());
    args.push("--alias".into());
    args.push(profile.model.clone().into());
    args.push("--api-key-file".into());
    args.push(key_file.into());
    let mut push = |flag: &str, value: Option<std::ffi::OsString>| {
        if let Some(value) = value {
            args.push(flag.into());
            args.push(value);
        }
    };
    push("--mmproj", settings.mmproj.as_ref().map(Into::into));
    push(
        "--n-cpu-moe",
        settings.n_cpu_moe.map(|n| n.to_string().into()),
    );
    push(
        "--override-tensor",
        settings.override_tensor.clone().map(Into::into),
    );
    push(
        "--tensor-split",
        settings.tensor_split.clone().map(Into::into),
    );
    push(
        "--main-gpu",
        settings.main_gpu.map(|n| n.to_string().into()),
    );
    push(
        "--rope-scaling",
        settings.rope_scaling.clone().map(Into::into),
    );
    push(
        "--rope-scale",
        settings.rope_scale.map(|n| n.to_string().into()),
    );
    push(
        "--yarn-orig-ctx",
        settings.yarn_orig_ctx.map(|n| n.to_string().into()),
    );
    push(
        "--chat-template-file",
        settings.chat_template_file.as_ref().map(Into::into),
    );
    push(
        "--model-draft",
        settings.draft_model.as_ref().map(Into::into),
    );
    push(
        "--draft-max",
        settings.draft_max.map(|n| n.to_string().into()),
    );
    if settings.mlock {
        args.push("--mlock".into());
    }
    if settings.no_mmap {
        args.push("--no-mmap".into());
    }
    if let Some(enabled) = settings.thinking {
        args.push("--chat-template-kwargs".into());
        args.push(
            serde_json::json!({"enable_thinking":enabled})
                .to_string()
                .into(),
        );
    }
    args.extend(settings.extra_args.iter().map(Into::into));
    args
}

/// Download, verify and unpack a pinned runtime archive into Alt's tools
/// folder, keeping any previous copy until the new one is in place.
pub async fn install_pin(
    root: &Path,
    pin: &Pin,
    cancel: Cancel,
    mut progress: impl FnMut(Progress),
) -> Result<PathBuf> {
    ensure!(
        !pin.directory.is_empty()
            && Path::new(pin.directory)
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_))),
        "Pinned runtime directory must be a plain folder name"
    );
    let tools = root.join("tools");
    private_dir(&tools)?;
    let archive = tools.join(format!("{}.tar.gz", pin.sha256));
    models::download_verified(
        pin.url,
        &archive,
        pin.size,
        pin.sha256,
        cancel.clone(),
        &mut progress,
    )
    .await?;
    ensure!(!cancel.load(Ordering::Relaxed), "Installation cancelled");
    progress(Progress {
        received: pin.size,
        total: pin.size,
        stage: format!("Installing verified {}", pin.label),
    });
    let directory = pin.directory.to_owned();
    tokio::task::spawn_blocking(move || -> Result<PathBuf> {
        let temp = tempfile::tempdir_in(&tools)?;
        let gzip = flate2::read::GzDecoder::new(std::fs::File::open(&archive)?);
        tar::Archive::new(gzip)
            .unpack(temp.path())
            .context("Could not unpack the verified runtime")?;
        let unpacked = temp.path().join(&directory);
        ensure!(
            unpacked.join("llama-server").is_file(),
            "The verified archive has no {directory}/llama-server"
        );
        let destination = tools.join(&directory);
        let previous = temp.path().join("previous");
        if destination.exists() {
            std::fs::rename(&destination, &previous)?;
        }
        if let Err(error) = std::fs::rename(&unpacked, &destination) {
            if previous.exists() {
                std::fs::rename(&previous, &destination)?;
            }
            return Err(error.into());
        }
        Ok(destination.join("llama-server"))
    })
    .await?
}

pub struct LocalRuntime {
    child: Child,
    pub endpoint: String,
    /// Random per-launch credential. Other local accounts can reach the loopback
    /// port but cannot use it without this key; it is never written to profiles.
    pub api_key: String,
    log: PathBuf,
}

impl LocalRuntime {
    pub async fn start(
        root: &Path,
        preferences: &Preferences,
        profile: &Profile,
        cancel: Cancel,
        progress: impl FnMut(Progress),
    ) -> Result<Self> {
        Self::start_observed(root, preferences, profile, cancel, progress, |_| {}).await
    }
    pub async fn start_observed(
        root: &Path,
        preferences: &Preferences,
        profile: &Profile,
        cancel: Cancel,
        mut progress: impl FnMut(Progress),
        mut started: impl FnMut(u32),
    ) -> Result<Self> {
        let id = profile
            .local_model
            .as_deref()
            .context("No local model selected")?;
        let artifact = models::artifact(root, id)?;
        preferences.runtime.validate()?;
        let detected = hardware().await;
        ensure!(
            detected.available_ram == 0
                || detected.available_ram > artifact.bytes.saturating_add(512 * 1024 * 1024),
            "Available RAM is below this file size plus minimal runtime headroom. Select a smaller model, close other applications, or connect to an external runtime. Actual context memory needs may be higher."
        );
        models::verify_artifact(&artifact, &cancel, &mut progress).await?;
        let binary = find_runtime(root, preferences).with_context(|| match &preferences.runtime_path {
            Some(selected) => format!(
                "The selected llama-server {} is missing or not executable. Alt does not substitute another runtime: choose it again or clear it in Settings → Runtime",
                selected.display()
            ),
            None => "Install the local runtime from Models, or set llama-server in Settings".into(),
        })?;
        if preferences.runtime.gpu_layers != 0 {
            let devices = tokio::time::timeout(
                Duration::from_secs(10),
                Command::new(&binary)
                    .arg("--list-devices")
                    .kill_on_drop(true)
                    .output(),
            )
            .await
            .context("GPU device discovery timed out; use CPU layers 0 or inspect the runtime")??;
            let output = format!(
                "{}\n{}",
                String::from_utf8_lossy(&devices.stdout),
                String::from_utf8_lossy(&devices.stderr)
            );
            ensure!(
                devices.status.success()
                    && output
                        .lines()
                        .any(|line| line.contains("MiB") && line.contains(':')),
                "GPU layers were requested but this runtime did not report a usable accelerator. Set GPU layers to 0 for CPU, or select a compatible GPU build in Settings. Discovery: {}",
                crate::display_text(&output)
                    .chars()
                    .take(2000)
                    .collect::<String>()
            );
        }
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
        let port = listener.local_addr()?.port();
        drop(listener);
        let logs = root.join("logs");
        private_dir(&logs)?;
        let log = logs.join(format!("runtime-{}.log", uuid::Uuid::new_v4()));
        let output = std::fs::File::create(&log)?;
        // The key goes through a private file, not argv, so `ps` cannot reveal it.
        let api_key = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let secrets = root.join("run");
        private_dir(&secrets)?;
        let key_file = tempfile::Builder::new()
            .prefix("runtime-key-")
            .tempfile_in(&secrets)?;
        std::fs::write(key_file.path(), &api_key)?;
        let threads = if preferences.runtime.threads > 0 {
            preferences.runtime.threads
        } else {
            std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(2)
                .min(8)
        };
        let mut command = Command::new(&binary);
        command
            .args(server_args(
                &preferences.runtime,
                profile,
                &artifact.path,
                port,
                threads,
                key_file.path(),
            ))
            .stdin(Stdio::null())
            .stdout(output.try_clone()?)
            .stderr(output)
            .kill_on_drop(true);
        // Managed archives carry their shared libraries beside llama-server.
        if binary.starts_with(root.join("tools"))
            && let Some(directory) = binary.parent()
        {
            let mut paths = vec![directory.to_path_buf()];
            if let Some(existing) = std::env::var_os("LD_LIBRARY_PATH") {
                paths.extend(std::env::split_paths(&existing));
            }
            command.env("LD_LIBRARY_PATH", std::env::join_paths(paths)?);
        }
        if let Some(tokens) = profile.effective_inference().reasoning_tokens {
            // Fail visibly on older runtimes rather than claiming an unsupported allocation.
            let help = tokio::time::timeout(
                Duration::from_secs(5),
                Command::new(&binary)
                    .arg("--help")
                    .kill_on_drop(true)
                    .output(),
            )
            .await
            .context("Runtime capability probe timed out")??;
            ensure!(
                String::from_utf8_lossy(&help.stdout).contains("--reasoning-budget"),
                "Selected runtime does not support --reasoning-budget; upgrade or clear this setting"
            );
            command.arg("--reasoning-budget").arg(tokens.to_string());
        }
        crate::process::configure(&mut command);
        let child = command
            .spawn()
            .context("The local runtime could not start. Check its executable in Settings")?;
        if let Some(pid) = child.id() {
            started(pid);
        }
        let mut runtime = Self {
            child,
            endpoint: format!("http://127.0.0.1:{port}/v1"),
            api_key,
            log,
        };
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(1))
            .build()?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
        while tokio::time::Instant::now() < deadline {
            ensure!(!cancel.load(Ordering::Relaxed), "Model loading cancelled");
            if let Some(status) = runtime.child.try_wait()? {
                let details = runtime.log_tail();
                if details.contains("error while loading shared libraries") {
                    bail!(
                        "Local runtime could not load a required system library. On Debian/Ubuntu, libgomp.so.1 is supplied by libgomp1 and libnuma.so.1 by libnuma1; install the package named by the error or choose a runtime compatible with your system in Settings. The selected model was not changed. Details: {details}"
                    );
                }
                bail!(
                    "Local runtime stopped ({status}). If GPU initialization failed, choose a compatible runtime in Settings or select CPU (0 GPU layers); for allocation failures reduce context, batch or layers. The selected model was not changed. Details: {details}"
                );
            }
            // Readiness needs an authenticated model listing that names the
            // selected alias, not only an open health route on a reused port.
            if let Ok(response) = client
                .get(format!("http://127.0.0.1:{port}/health"))
                .send()
                .await
                && response.status().is_success()
                && runtime.owns_endpoint(&client).await
            {
                drop(key_file);
                progress(Progress {
                    received: 1,
                    total: 1,
                    stage: "Local model is ready".into(),
                });
                return Ok(runtime);
            }
            progress(Progress {
                received: 0,
                total: 0,
                stage: format!(
                    "Loading model ({} GPU layers requested)",
                    preferences.runtime.gpu_layers
                ),
            });
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        bail!(
            "The model did not become ready within two minutes. Try a smaller model or inspect {}",
            runtime.log.display()
        )
    }
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }
    /// Point a profile at this runtime, including its per-launch key.
    pub fn attach(&self, profile: &mut Profile) {
        profile.endpoint = self.endpoint.clone();
        profile.runtime_key = Some(self.api_key.clone());
    }
    async fn owns_endpoint(&self, client: &reqwest::Client) -> bool {
        let Ok(response) = client
            .get(format!("{}/models", self.endpoint))
            .bearer_auth(&self.api_key)
            .send()
            .await
        else {
            return false;
        };
        if !response.status().is_success() {
            return false;
        }
        response
            .json::<serde_json::Value>()
            .await
            .is_ok_and(|v| v["data"].as_array().is_some_and(|rows| !rows.is_empty()))
    }
    pub fn log_path(&self) -> &Path {
        &self.log
    }
    fn log_tail(&self) -> String {
        use std::io::{Read, Seek, SeekFrom};
        let Ok(mut file) = std::fs::File::open(&self.log) else {
            return String::new();
        };
        let _ = file.seek(SeekFrom::End(-2048));
        let mut text = String::new();
        let _ = file.read_to_string(&mut text);
        crate::display_text(&text)
    }
    pub async fn stop(&mut self) {
        self.kill();
        let _ = self.child.wait().await;
    }
    fn kill(&mut self) {
        #[cfg(unix)]
        if let Some(id) = self.child.id() {
            let _ = nix::sys::signal::killpg(
                nix::unistd::Pid::from_raw(id as i32),
                nix::sys::signal::Signal::SIGKILL,
            );
        }
        let _ = self.child.start_kill();
    }
}
impl Drop for LocalRuntime {
    fn drop(&mut self) {
        self.kill();
    }
}
