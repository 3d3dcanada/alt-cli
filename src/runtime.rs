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
    preferences
        .engine_path
        .as_deref()
        .and_then(find_executable)
        .or_else(|| find_executable(&root.join("tools/goose")))
        .or_else(|| find_executable(Path::new("goose")))
        .or_else(|| find_executable(Path::new("/workspace/.alt-tools/goose")))
}

pub fn find_runtime(root: &Path, preferences: &Preferences) -> Option<PathBuf> {
    preferences
        .runtime_path
        .as_deref()
        .and_then(find_executable)
        .or_else(|| find_executable(&root.join("tools/llama-b11429/llama-server")))
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
    pub endpoint: String,
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
        let binary = find_runtime(root, preferences)
            .context("Install the local runtime from Models, or set llama-server in Settings")?;
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
        let threads = if preferences.runtime.threads > 0 {
            preferences.runtime.threads
        } else {
            std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(2)
                .min(8)
        };
        let mut command = Command::new(binary);
        command
            .args(["--model"])
            .arg(&artifact.path)
            .args([
                "--ctx-size",
                &profile.context_tokens.to_string(),
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
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
            if let Ok(response) = client
                .get(format!("http://127.0.0.1:{port}/health"))
                .send()
                .await
                && response.status().is_success()
            {
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
