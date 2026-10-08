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
#[path = "runtime_endpoint.rs"]
mod private_endpoint;

/// Owned loopback capability URLs must never leave this machine via an inherited
/// HTTP proxy. External endpoints retain the operator's proxy configuration.
pub fn http_client(profile: &Profile) -> reqwest::ClientBuilder {
    let builder = reqwest::Client::builder();
    if profile.local_model.is_some() {
        builder.no_proxy()
    } else {
        builder
    }
}

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
    pub startup_timeout_secs: u64,
}
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
            startup_timeout_secs: 180,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (5..=3600).contains(&self.startup_timeout_secs),
            "Model loading deadline must be 5–3600 seconds"
        );
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
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Hardware {
    pub ram: u64,
    pub available_ram: u64,
    pub threads: usize,
    pub gpu: String,
    pub vram_total: Option<u64>,
    pub vram_free: Option<u64>,
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
        vram_total: None,
        vram_free: None,
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
    let result = crate::process::bounded_output(
        Command::new("nvidia-smi").args([
            "--query-gpu=name,memory.total,memory.free",
            "--format=csv,noheader,nounits",
        ]),
        Duration::from_secs(2),
        64 * 1024,
    )
    .await;
    if let Ok(result) = result
        && result.status.success()
    {
        data.gpu = crate::display_text(String::from_utf8_lossy(&result.stdout).trim());
        // Conservative single-device guidance; never sum disjoint VRAM budgets.
        if let Some(line) = String::from_utf8_lossy(&result.stdout).lines().next() {
            let fields: Vec<_> = line.rsplitn(3, ',').map(str::trim).collect();
            data.vram_free = fields
                .first()
                .and_then(|s| s.parse::<u64>().ok())
                .and_then(|n| n.checked_mul(1024 * 1024));
            data.vram_total = fields
                .get(1)
                .and_then(|s| s.parse::<u64>().ok())
                .and_then(|n| n.checked_mul(1024 * 1024));
        }
    }
    data
}

pub fn find_engine(root: &Path, requested: &Path, preferences: &Preferences) -> Option<PathBuf> {
    if requested != Path::new("goose") {
        return find_executable(requested);
    }
    if let Some(path) = preferences.engine_path.as_deref() {
        return find_executable(path);
    }
    find_executable(&root.join("tools/goose"))
        .or_else(|| find_executable(Path::new("goose")))
        .or_else(|| find_executable(Path::new("/workspace/.alt-tools/goose")))
}

pub fn find_runtime(root: &Path, preferences: &Preferences) -> Option<PathBuf> {
    if let Some(path) = preferences.runtime_path.as_deref() {
        return find_executable(path);
    }
    find_executable(&root.join("tools/llama-b11429/llama-server"))
        .or_else(|| find_executable(Path::new("llama-server")))
        .or_else(|| {
            find_executable(Path::new(
                "/workspace/.alt-tools/llama-b11429/llama-b11429/llama-server",
            ))
        })
}

pub fn find_executable(path: &Path) -> Option<PathBuf> {
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
}

impl Component {
    pub fn description(self) -> &'static str {
        match self {
            Self::Engine => "Goose 1.53.0 agent engine",
            Self::Inference => "llama.cpp b11429 CPU runtime",
        }
    }
    pub fn size(self) -> u64 {
        match self {
            Self::Engine => 94_629_008,
            Self::Inference => 17_693_462,
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
        }
    }
}

pub async fn install(
    root: &Path,
    component: Component,
    cancel: Cancel,
    mut progress: impl FnMut(Progress),
) -> Result<PathBuf> {
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

pub struct LocalRuntime {
    child: Child,
    group: crate::process::OwnedGroup,
    pub endpoint: String,
    log: PathBuf,
    _private_endpoint: private_endpoint::Endpoint,
    lease: Option<std::fs::File>,
    source_identity: Vec<(PathBuf, FileIdentity)>,
}
#[derive(PartialEq, Eq)]
struct FileIdentity {
    bytes: u64,
    modified: std::time::SystemTime,
    #[cfg(unix)]
    dev: u64,
    #[cfg(unix)]
    ino: u64,
    #[cfg(unix)]
    ctime: (i64, i64),
}
fn file_identity(path: &Path) -> Result<FileIdentity> {
    let m = std::fs::metadata(path)?;
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;
    Ok(FileIdentity {
        bytes: m.len(),
        modified: m.modified()?,
        #[cfg(unix)]
        dev: m.dev(),
        #[cfg(unix)]
        ino: m.ino(),
        #[cfg(unix)]
        ctime: (m.ctime(), m.ctime_nsec()),
    })
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
        private_dir(root)?;
        let lease = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join("owned-runtime.lock"))?;
        fs2::FileExt::try_lock_exclusive(&lease).context("Another owned model is already running for this Alt state. Disconnect it before loading or benchmarking another model")?;
        let detected = hardware().await;
        ensure!(
            detected.available_ram == 0
                || detected.available_ram > artifact.bytes.saturating_add(512 * 1024 * 1024),
            "Available RAM is below this file size plus minimal runtime headroom. Select a smaller model, close other applications, or connect to an external runtime. Actual context memory needs may be higher."
        );
        models::verify_artifact(&artifact, &cancel, &mut progress).await?;
        let binary = find_runtime(root, preferences)
            .with_context(|| match &preferences.runtime_path {
                Some(path)=>format!("Selected runtime {} is missing or not executable. Restore it or explicitly choose another runtime in Settings; no fallback was launched",path.display()),
                None=>"Install the local runtime from Models, or set llama-server in Settings".into(),
            })?;
        let mut paths = if artifact.pieces.is_empty() {
            vec![artifact.path.clone()]
        } else {
            artifact.pieces.iter().map(|p| p.path.clone()).collect()
        };
        paths.push(binary.clone());
        let source_identity = paths
            .into_iter()
            .map(|p| Ok((p.clone(), file_identity(&p)?)))
            .collect::<Result<Vec<_>>>()?;
        if preferences.runtime.gpu_layers != 0 {
            let devices = crate::process::bounded_output(
                Command::new(&binary).arg("--list-devices"),
                Duration::from_secs(10),
                1024 * 1024,
            )
            .await
            .context("GPU device discovery failed; use CPU layers 0 or inspect the runtime")?;
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
        let private_endpoint = private_endpoint::Endpoint::reserve().await?;
        let logs = root.join("logs");
        private_dir(&logs)?;
        let log = logs.join(format!("runtime-{}.log", uuid::Uuid::new_v4()));
        let output = std::fs::File::create(&log)?;
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
            .args(["--model"])
            .arg(&artifact.path)
            .args([
                "--ctx-size",
                &profile.context_tokens.to_string(),
                "--host",
                private_endpoint
                    .socket
                    .to_str()
                    .context("Private runtime socket path is not UTF-8")?,
                "--port",
                "0",
                "--threads",
                &threads.to_string(),
                "--parallel",
                "1",
                "--n-gpu-layers",
                &preferences.runtime.gpu_layers.to_string(),
                "--batch-size",
                &preferences.runtime.batch.to_string(),
                "--ubatch-size",
                &preferences.runtime.batch.min(512).to_string(),
                "--cache-type-k",
                &preferences.runtime.cache_k,
                "--cache-type-v",
                &preferences.runtime.cache_v,
                "--flash-attn",
                if preferences.runtime.flash_attention {
                    "on"
                } else {
                    "off"
                },
                "--jinja",
                "--alias",
                &profile.model,
            ])
            .stdin(Stdio::null())
            .stdout(output.try_clone()?)
            .stderr(output)
            .kill_on_drop(true);
        if let Some(enabled) = preferences.runtime.thinking {
            command.args([
                "--chat-template-kwargs",
                &serde_json::json!({"enable_thinking":enabled}).to_string(),
            ]);
        }
        if let Some(tokens) = profile.effective_inference().reasoning_tokens {
            // Fail visibly on older runtimes rather than claiming an unsupported allocation.
            let help = crate::process::bounded_output(
                Command::new(&binary).arg("--help"),
                Duration::from_secs(5),
                1024 * 1024,
            )
            .await
            .context("Runtime capability probe failed")?;
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
        let group = crate::process::OwnedGroup::capture(&child);
        if let Some(pid) = child.id() {
            started(pid);
        }
        let mut runtime = Self {
            child,
            group,
            endpoint: private_endpoint.url.clone(),
            log,
            _private_endpoint: private_endpoint,
            lease: Some(lease),
            source_identity,
        };
        let client = runtime._private_endpoint.client()?;
        let deadline = tokio::time::Instant::now()
            + Duration::from_secs(preferences.runtime.startup_timeout_secs);
        while tokio::time::Instant::now() < deadline {
            if cancel.load(Ordering::Relaxed) {
                runtime.stop().await;
                bail!("Model loading cancelled");
            }
            if let Some(status) = runtime.child.try_wait()? {
                let details = runtime.log_tail();
                runtime.stop().await;
                if details.contains("error while loading shared libraries") {
                    bail!(
                        "Local runtime could not load a required system library. On Debian/Ubuntu, libgomp.so.1 is supplied by libgomp1 and libnuma.so.1 by libnuma1; install the package named by the error or choose a runtime compatible with your system in Settings. The selected model was not changed. Details: {details}"
                    );
                }
                bail!(
                    "Local runtime stopped ({status}). If GPU initialization failed, choose a compatible runtime in Settings or select CPU (0 GPU layers); for allocation failures reduce context, batch or layers. The selected model was not changed. Details: {details}"
                );
            }
            if let Ok(response) = client.get("http://localhost/health").send().await
                && response.status().is_success()
            {
                runtime.check_warm_identity()?;
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
        runtime.stop().await;
        bail!(
            "The model did not become ready within {} seconds. Increase the loading deadline in Runtime settings if this disk/CPU needs longer, or inspect {}",
            preferences.runtime.startup_timeout_secs,
            runtime.log.display()
        )
    }
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }
    /// Reuse is limited to this already verified owned process. Reconnecting
    /// always performs full verification; changed on-disk identities never
    /// silently inherit the earlier integrity result.
    pub fn check_warm_identity(&mut self) -> Result<()> {
        ensure!(
            self.child.try_wait()?.is_none(),
            "Owned runtime exited; reconnect before continuing"
        );
        for (path, original) in &self.source_identity {
            ensure!(
                &file_identity(path)? == original,
                "Model or runtime file changed while loaded: {}. Disconnect, verify the selected files and reconnect before reuse",
                path.display()
            );
        }
        Ok(())
    }
    pub fn log_path(&self) -> &Path {
        &self.log
    }
    pub fn offload_observation(&self) -> serde_json::Value {
        use std::io::{Read, Seek, SeekFrom};
        let observed = (|| -> Result<serde_json::Value> {
            let mut file = std::fs::File::open(&self.log)?;
            let length = file.metadata()?.len();
            file.seek(SeekFrom::Start(length.saturating_sub(256 * 1024)))?;
            let mut text = String::new();
            file.take(256 * 1024).read_to_string(&mut text)?;
            let pattern = regex::Regex::new(r"offloaded (\d+)/(\d+) layers to GPU")?;
            if let Some(c) = pattern.captures(&text) {
                Ok(
                    serde_json::json!({"offloaded_layers":c[1].parse::<u32>()?,"total_layers":c[2].parse::<u32>()?,"source":"Owned runtime log; reported by the selected runtime, not inferred from requested layers"}),
                )
            } else {
                Ok(
                    serde_json::json!({"offloaded_layers":null,"source":"Selected runtime did not emit a recognized layer count; requested GPU settings are not a measurement"}),
                )
            }
        })();
        observed
            .unwrap_or_else(|e| serde_json::json!({"offloaded_layers":null,"error":e.to_string()}))
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
        self.group.stop(&mut self.child).await;
        self.lease.take();
    }
    fn kill(&mut self) {
        self.group.kill();
        let _ = self.child.start_kill();
    }
}
impl Drop for LocalRuntime {
    fn drop(&mut self) {
        self.kill();
    }
}
