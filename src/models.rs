//! Provider inventory, public Hub discovery, and verified local model files.
use crate::config::{Profile, Provider, atomic_write, private_dir};
use anyhow::{Context, Result, bail, ensure};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[path = "gguf.rs"]
pub mod gguf;

#[derive(Debug, Clone, Default)]
pub struct Progress {
    pub received: u64,
    pub total: u64,
    pub stage: String,
}
pub type Cancel = Arc<AtomicBool>;

pub async fn cancelled(cancel: &Cancel) {
    while !cancel.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

pub fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .user_agent(concat!("alt-cli/", env!("CARGO_PKG_VERSION")))
        .build()?)
}

/// Hub credentials are attached only to the exact HTTPS Hub origin. Reqwest
/// strips Authorization on cross-origin CDN redirects; generic endpoints are separate.
pub fn hub_request(request: reqwest::RequestBuilder, url: &str) -> Result<reqwest::RequestBuilder> {
    let url = reqwest::Url::parse(url)?;
    if url.scheme() == "https"
        && url.host_str() == Some("huggingface.co")
        && url.port_or_known_default() == Some(443)
        && let Ok(token) = std::env::var("HF_TOKEN")
    {
        ensure!(!token.trim().is_empty(), "HF_TOKEN is empty");
        return Ok(request.bearer_auth(token));
    }
    Ok(request)
}
async fn json_response(request: reqwest::RequestBuilder) -> Result<Value> {
    let response = request.timeout(Duration::from_secs(20)).send().await
        .context("Could not connect. Check the address, internet connection, and whether the server is running")?
        .error_for_status().context("The server refused the request. Check its address and authentication settings")?;
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        bytes.extend_from_slice(&chunk?);
        ensure!(
            bytes.len() <= 8 * 1024 * 1024,
            "Server response is too large"
        );
    }
    serde_json::from_slice(&bytes).context("The server returned an unreadable model list")
}

pub async fn inventory(profile: &Profile) -> Result<Vec<String>> {
    let mut request = crate::runtime::http_client(profile)
        .connect_timeout(Duration::from_secs(10))
        .build()?
        .get(profile.models_url());
    if let Some(key) = profile.key()? {
        request = request.bearer_auth(key);
    }
    let value = json_response(request).await?;
    let (field, name) = match profile.provider {
        Provider::Openai => ("data", "id"),
        Provider::Ollama => ("models", "name"),
    };
    let mut models: Vec<String> = value[field]
        .as_array()
        .context("This address did not return a supported model list")?
        .iter()
        .filter_map(|model| model[name].as_str().map(str::to_owned))
        .collect();
    models.sort();
    models.dedup();
    Ok(models)
}

#[derive(Debug, Clone, Deserialize)]
pub struct HubModel {
    pub id: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub tags: Vec<String>,
}

pub async fn search(query: &str, variants_only: bool) -> Result<Vec<HubModel>> {
    let value = json_response(
        hub_request(
            client()?.get("https://huggingface.co/api/models"),
            "https://huggingface.co/api/models",
        )?
        .query(&[
            ("search", query),
            ("filter", "gguf"),
            ("limit", "60"),
            ("sort", "downloads"),
            ("direction", "-1"),
        ]),
    )
    .await?;
    let models: Vec<HubModel> = serde_json::from_value(value)?;
    Ok(models
        .into_iter()
        .filter(|model| {
            !variants_only || variant_claim(&format!("{} {}", model.id, model.tags.join(" ")))
        })
        .collect())
}

fn variant_claim(text: &str) -> bool {
    let text = text.to_lowercase();
    ["uncensored", "abliterat", "heretic", "decensor"]
        .iter()
        .any(|word| text.contains(word))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelFile {
    pub repo: String,
    pub revision: String,
    pub filename: String,
    pub bytes: u64,
    pub sha256: String,
    pub license: String,
    pub uncensored_claim: bool,
    #[serde(default)]
    pub parts: Vec<ModelPart>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPart {
    pub filename: String,
    pub bytes: u64,
    pub sha256: String,
}

impl ModelFile {
    pub fn url(&self) -> Result<String> {
        let mut url = reqwest::Url::parse("https://huggingface.co")?;
        url.path_segments_mut()
            .map_err(|_| anyhow::anyhow!("Invalid Hub base"))?
            .extend(self.repo.split('/'))
            .push("resolve")
            .push(&self.revision)
            .extend(self.filename.split('/'));
        Ok(url.into())
    }
}

pub async fn files(repo: &str) -> Result<Vec<ModelFile>> {
    ensure!(
        repo.split('/').count() == 2
            && repo.split('/').all(|s| !s.is_empty()
                && s != "."
                && s != ".."
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))),
        "Enter a repository as publisher/model-name"
    );
    let url = format!("https://huggingface.co/api/models/{repo}");
    let value =
        json_response(hub_request(client()?.get(&url), &url)?.query(&[("blobs", "true")])).await?;
    catalog(repo, &value)
}

/// Group exact split GGUF sets; incomplete sets are not installable choices.
pub fn catalog(repo: &str, value: &Value) -> Result<Vec<ModelFile>> {
    let revision = value["sha"]
        .as_str()
        .context("The Hub did not provide an immutable revision")?;
    let license = value["cardData"]["license"]
        .as_str()
        .unwrap_or("Not declared — review the model card");
    let uncensored_claim = variant_claim(&format!("{repo} {}", value["tags"]));
    let mut files = Vec::new();
    for file in value["siblings"]
        .as_array()
        .context("The Hub did not return any files")?
    {
        let Some(name) = file["rfilename"].as_str() else {
            continue;
        };
        if !name.ends_with(".gguf") || name.to_lowercase().contains("mmproj") {
            continue;
        }
        let (Some(hash), Some(bytes)) = (file["lfs"]["sha256"].as_str(), file["size"].as_u64())
        else {
            continue;
        };
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        files.push(ModelFile {
            repo: repo.into(),
            revision: revision.into(),
            filename: name.into(),
            bytes,
            sha256: hash.into(),
            license: license.into(),
            uncensored_claim,
            parts: vec![],
        });
    }
    let pattern = regex::Regex::new(r"^(.*)-(\d{5})-of-(\d{5})\.gguf$")?;
    let mut result = Vec::new();
    for f in &files {
        if let Some(c) = pattern.captures(&f.filename) {
            if &c[2] != "00001" {
                continue;
            }
            let count: usize = c[3].parse()?;
            ensure!(count > 0 && count <= 1024, "Unreasonable GGUF split count");
            let mut parts = Vec::new();
            for n in 1..=count {
                let filename = format!("{}-{n:05}-of-{count:05}.gguf", &c[1]);
                if let Some(part) = files.iter().find(|p| p.filename == filename) {
                    parts.push(ModelPart {
                        filename,
                        bytes: part.bytes,
                        sha256: part.sha256.clone(),
                    });
                }
            }
            if parts.len() == count {
                let mut grouped = f.clone();
                grouped.bytes = parts
                    .iter()
                    .try_fold(0u64, |n, p| n.checked_add(p.bytes))
                    .context("Model set size overflow")?;
                grouped.parts = parts;
                result.push(grouped);
            }
        } else {
            result.push(f.clone());
        }
    }
    result.sort_by_key(|file| file.bytes);
    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub bytes: u64,
    pub sha256: String,
    pub license: String,
    pub uncensored_claim: bool,
    pub source: Option<ModelFile>,
    #[serde(default)]
    pub pieces: Vec<LocalPart>,
    #[serde(default)]
    pub metadata: Option<gguf::Metadata>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalPart {
    pub path: PathBuf,
    pub bytes: u64,
    pub sha256: String,
}

pub fn library(root: &Path) -> Result<Vec<Artifact>> {
    let dir = root.join("models");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "json") {
            let model: Artifact =
                serde_json::from_slice(&std::fs::read(&path)?).with_context(|| {
                    format!(
                        "The model record {} is damaged; your weights are untouched",
                        path.display()
                    )
                })?;
            entries.push(model);
        }
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

pub fn artifact(root: &Path, id: &str) -> Result<Artifact> {
    ensure!(
        id.len() == 64 && id.chars().all(|c| c.is_ascii_hexdigit()),
        "Invalid local model identifier"
    );
    Ok(serde_json::from_slice(
        &std::fs::read(root.join("models").join(format!("{id}.json"))).context(
            "This local model is no longer in your library. Open Models to import it again",
        )?,
    )?)
}

pub async fn hash_file(
    path: &Path,
    cancel: &Cancel,
    mut progress: impl FnMut(Progress),
) -> Result<(u64, String)> {
    let mut file = tokio::fs::File::open(path).await?;
    let total = file.metadata().await?.len();
    let mut hash = Sha256::new();
    let mut count = 0;
    let mut bytes = vec![0u8; 1024 * 1024];
    let mut last = Instant::now();
    loop {
        ensure!(
            !cancel.load(Ordering::Relaxed),
            "Cancelled; your existing files were kept"
        );
        let n = file.read(&mut bytes).await?;
        if n == 0 {
            break;
        }
        hash.update(&bytes[..n]);
        count += n as u64;
        if last.elapsed() > Duration::from_millis(150) {
            progress(Progress {
                received: count,
                total,
                stage: "Checking file integrity".into(),
            });
            last = Instant::now();
        }
    }
    Ok((count, format!("{:x}", hash.finalize())))
}

pub async fn verify_gguf(path: &Path) -> Result<()> {
    inspect_gguf(path).await.map(|_| ())
}
pub async fn inspect_gguf(path: &Path) -> Result<gguf::Metadata> {
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || gguf::inspect(&path)).await?
}
fn validate_set(metadata: &[gguf::Metadata]) -> Result<()> {
    let first = metadata.first().context("Empty GGUF set")?;
    let expected = first.split_count.unwrap_or(1);
    ensure!(
        expected as usize == metadata.len(),
        "Incomplete GGUF split set: expected {expected} members, found {}. Import the complete set again",
        metadata.len()
    );
    let mut tensors = 0u64;
    for (index, m) in metadata.iter().enumerate() {
        ensure!(
            m.split_count.unwrap_or(1) == expected
                && (expected == 1 || m.split_index == Some(index as u64)),
            "GGUF split indexes/counts do not match the numbered set"
        );
        ensure!(
            m.architecture == first.architecture
                && m.blocks == first.blocks
                && m.embedding == first.embedding
                && m.native_context == first.native_context
                && m.total_tensors == first.total_tensors,
            "GGUF split members have inconsistent model metadata"
        );
        tensors = tensors
            .checked_add(m.tensors)
            .context("GGUF tensor count overflow")?;
    }
    ensure!(
        first.total_tensors.is_none_or(|n| n == tensors),
        "GGUF split tensor total does not match its metadata"
    );
    Ok(())
}

pub async fn import(
    root: &Path,
    path: &Path,
    uncensored_claim: bool,
    cancel: Cancel,
    mut progress: impl FnMut(Progress),
) -> Result<Artifact> {
    let path = path.canonicalize().context("That file does not exist")?;
    let header = inspect_gguf(&path).await?;
    let pattern = regex::Regex::new(r"^(.*)-(\d{5})-of-(\d{5})\.gguf$")?;
    let filename = path
        .file_name()
        .context("Model filename")?
        .to_string_lossy();
    let paths = if let Some(c) = pattern.captures(&filename) {
        let count: u64 = c[3].parse()?;
        ensure!(
            (1..=1024).contains(&count) && header.split_count == Some(count),
            "Numbered GGUF filename and split metadata disagree"
        );
        let parent = path.parent().context("Model parent")?;
        (1..=count)
            .map(|n| parent.join(format!("{}-{n:05}-of-{count:05}.gguf", &c[1])))
            .collect::<Vec<_>>()
    } else {
        ensure!(
            header.split_count.unwrap_or(1) == 1,
            "Split GGUF files require their original numbered filenames and complete set"
        );
        vec![path.clone()]
    };
    let mut pieces = Vec::new();
    let mut metadata = Vec::new();
    let mut bytes = 0u64;
    for member in paths {
        ensure!(!cancel.load(Ordering::Relaxed), "Import cancelled");
        let member = member
            .canonicalize()
            .with_context(|| format!("Missing GGUF split member {}", member.display()))?;
        metadata.push(inspect_gguf(&member).await?);
        let (size, hash) = hash_file(&member, &cancel, &mut progress).await?;
        bytes = bytes.checked_add(size).context("GGUF set size overflow")?;
        pieces.push(LocalPart {
            path: member,
            bytes: size,
            sha256: hash,
        });
    }
    validate_set(&metadata)?;
    let path = pieces[0].path.clone();
    let hash = pieces[0].sha256.clone();
    let id = if pieces.len() == 1 {
        hash.clone()
    } else {
        crate::project::digest(&serde_json::to_vec(&pieces)?)
    };
    let model = Artifact {
        id,
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        path,
        bytes,
        sha256: hash,
        license: "Imported file — check its publisher's license".into(),
        uncensored_claim,
        source: None,
        pieces,
        metadata: metadata.into_iter().next(),
    };
    save_artifact(root, &model)?;
    Ok(model)
}

fn save_artifact(root: &Path, model: &Artifact) -> Result<()> {
    atomic_write(
        &root.join("models").join(format!("{}.json", model.id)),
        &serde_json::to_vec_pretty(model)?,
    )
}

pub async fn download(
    root: &Path,
    file: ModelFile,
    cancel: Cancel,
    progress: impl FnMut(Progress),
) -> Result<Artifact> {
    download_set(root, file, cancel, progress, |f| f.url()).await
}

/// Common multipart transfer path, with an injectable origin for deterministic tests.
pub async fn download_set(
    root: &Path,
    file: ModelFile,
    cancel: Cancel,
    mut progress: impl FnMut(Progress),
    origin: impl Fn(&ModelFile) -> Result<String>,
) -> Result<Artifact> {
    let parts = if file.parts.is_empty() {
        vec![ModelPart {
            filename: file.filename.clone(),
            bytes: file.bytes,
            sha256: file.sha256.clone(),
        }]
    } else {
        file.parts.clone()
    };
    ensure!(
        parts.len() <= 1024 && !parts.is_empty(),
        "GGUF set must contain 1–1024 members"
    );
    ensure!(
        file.sha256.len() == 64 && file.sha256.chars().all(|c| c.is_ascii_hexdigit()),
        "Invalid GGUF artifact checksum"
    );
    ensure!(
        parts.iter().all(|p| p.bytes > 0
            && p.sha256.len() == 64
            && p.sha256.chars().all(|c| c.is_ascii_hexdigit())),
        "GGUF member has no usable size/checksum"
    );
    ensure!(
        parts[0].sha256.eq_ignore_ascii_case(&file.sha256) && parts[0].filename == file.filename,
        "GGUF first member does not match artifact metadata"
    );
    ensure!(
        parts.iter().try_fold(0u64, |n, p| n.checked_add(p.bytes)) == Some(file.bytes),
        "GGUF set size mismatch"
    );
    let id = if parts.len() == 1 {
        file.sha256.clone()
    } else {
        crate::project::digest(&serde_json::to_vec(&parts)?)
    };
    let cache = cache_root(root)?;
    private_dir(&cache)?;
    let final_dir = cache.join(&id);
    let staging = cache.join(format!("{id}.incomplete"));
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(cache.join(format!("{id}.set-lock")))?;
    fs2::FileExt::try_lock_exclusive(&lock)
        .context("This model set is already being transferred")?;
    let destination = if final_dir.exists() {
        &final_dir
    } else {
        &staging
    };
    private_dir(destination)?;
    let mut remaining = 0u64;
    let mut names = std::collections::BTreeSet::new();
    for part in &parts {
        ensure!(
            !part.filename.is_empty()
                && Path::new(&part.filename)
                    .components()
                    .all(|p| matches!(p, std::path::Component::Normal(_))),
            "Invalid model part path"
        );
        ensure!(names.insert(&part.filename), "Duplicate model part");
        let path = destination.join(&part.filename);
        let retained = std::fs::metadata(&path)
            .or_else(|_| std::fs::metadata(path.with_extension("partial")))
            .map(|m| m.len())
            .unwrap_or(0)
            .min(part.bytes);
        remaining = remaining
            .checked_add(part.bytes - retained)
            .context("Model set size overflow")?;
    }
    ensure!(
        fs2::available_space(destination)? >= remaining.saturating_add(64 * 1024 * 1024),
        "Not enough free disk space for the complete GGUF set; {} still required plus 64 MiB headroom",
        human_bytes(remaining)
    );
    let mut completed = 0;
    let mut headers = Vec::new();
    for part in &parts {
        ensure!(
            Path::new(&part.filename)
                .components()
                .all(|p| matches!(p, std::path::Component::Normal(_))),
            "Invalid model part path"
        );
        let mut metadata = file.clone();
        metadata.filename = part.filename.clone();
        metadata.bytes = part.bytes;
        metadata.sha256 = part.sha256.clone();
        metadata.parts.clear();
        let path = destination.join(&part.filename);
        download_verified(
            &origin(&metadata)?,
            &path,
            part.bytes,
            &part.sha256,
            cancel.clone(),
            |p| {
                progress(Progress {
                    received: completed + p.received,
                    total: file.bytes,
                    stage: format!("{} · {}", part.filename, p.stage),
                })
            },
        )
        .await?;
        headers.push(inspect_gguf(&path).await?);
        completed += part.bytes;
    }
    validate_set(&headers)?;
    if destination == &staging {
        tokio::fs::rename(&staging, &final_dir).await?;
        std::fs::File::open(&cache)?.sync_all()?;
    }
    let pieces: Vec<_> = parts
        .iter()
        .map(|p| LocalPart {
            path: final_dir.join(&p.filename),
            bytes: p.bytes,
            sha256: p.sha256.clone(),
        })
        .collect();
    let model = Artifact {
        id,
        name: file.filename.clone(),
        path: pieces[0].path.clone(),
        bytes: file.bytes,
        sha256: pieces[0].sha256.clone(),
        license: file.license.clone(),
        uncensored_claim: file.uncensored_claim,
        source: Some(file),
        pieces,
        metadata: headers.into_iter().next(),
    };
    save_artifact(root, &model)?;
    Ok(model)
}
pub fn cache_root(root: &Path) -> Result<PathBuf> {
    let config = root.join("model-cache.json");
    if config.exists() {
        Ok(serde_json::from_slice(&std::fs::read(config)?)?)
    } else {
        Ok(root.join("blobs"))
    }
}
pub async fn verify_artifact(
    model: &Artifact,
    cancel: &Cancel,
    mut progress: impl FnMut(Progress),
) -> Result<()> {
    let pieces = if model.pieces.is_empty() {
        vec![LocalPart {
            path: model.path.clone(),
            bytes: model.bytes,
            sha256: model.sha256.clone(),
        }]
    } else {
        model.pieces.clone()
    };
    let mut headers = Vec::new();
    for part in pieces {
        headers.push(inspect_gguf(&part.path).await?);
        let (bytes, hash) = hash_file(&part.path, cancel, &mut progress).await?;
        ensure!(
            bytes == part.bytes && hash.eq_ignore_ascii_case(&part.sha256),
            "Model part changed since import: {}. Restore a verified copy",
            part.path.display()
        );
    }
    validate_set(&headers)?;
    Ok(())
}
/// Removing an import unregisters it. Only recorded managed paths are deleted.
pub fn remove(root: &Path, id: &str, delete_weights: bool) -> Result<()> {
    let model = artifact(root, id)?;
    let config = crate::config::Config::load(root)?;
    ensure!(
        !config
            .profiles
            .values()
            .any(|p| p.local_model.as_deref() == Some(id)),
        "This model is selected by a connection. Remove or change that connection first"
    );
    if delete_weights && model.source.is_some() {
        let paths = if model.pieces.is_empty() {
            vec![model.path.clone()]
        } else {
            model.pieces.iter().map(|p| p.path.clone()).collect()
        };
        let allowed = cache_root(root)?.canonicalize().ok();
        let legacy = root.join("models").canonicalize().ok();
        for path in paths {
            if path.exists() {
                let canonical = path.canonicalize()?;
                ensure!(
                    allowed.as_ref().is_some_and(|p| canonical.starts_with(p))
                        || legacy.as_ref().is_some_and(|p| canonical.starts_with(p)),
                    "Managed path is outside model cache; preserved"
                );
                std::fs::remove_file(path)?;
            }
        }
    }
    std::fs::remove_file(root.join("models").join(format!("{id}.json")))?;
    Ok(())
}
/// Verified copy then atomic record switches. Originals are retained so an
/// interruption never destroys the only copy; the receipt lists cleanup candidates.
pub async fn relocate(root: &Path, destination: &Path, cancel: Cancel) -> Result<Value> {
    private_dir(destination)?;
    let destination = destination.canonicalize()?;
    let old = cache_root(root)?;
    ensure!(
        old.canonicalize().ok().as_ref() != Some(&destination)
            || root.join("cache-relocation.pending.json").exists(),
        "Cache already uses this folder"
    );
    let models = library(root)?;
    let journal_path = root.join("cache-relocation.pending.json");
    let mut journal: Value = if journal_path.exists() {
        let saved: Value = serde_json::from_slice(&std::fs::read(&journal_path)?)?;
        ensure!(
            saved["destination"] == serde_json::to_value(&destination)?,
            "A cache relocation is unfinished. Resume the destination recorded in cache-relocation.pending.json first; original weights are intact"
        );
        saved
    } else {
        let originals: Vec<_> = models
            .iter()
            .filter(|m| m.source.is_some())
            .flat_map(|m| {
                if m.pieces.is_empty() {
                    vec![m.path.clone()]
                } else {
                    m.pieces.iter().map(|p| p.path.clone()).collect()
                }
            })
            .collect();
        serde_json::json!({"schema":1,"destination":destination,"previous_cache":old,"originals":originals,"completed":[]})
    };
    atomic_write(&journal_path, &serde_json::to_vec_pretty(&journal)?)?;
    for mut m in models.into_iter().filter(|m| m.source.is_some()) {
        verify_artifact(&m, &cancel, |_| {}).await?;
        let pieces = if m.pieces.is_empty() {
            vec![LocalPart {
                path: m.path.clone(),
                bytes: m.bytes,
                sha256: m.sha256.clone(),
            }]
        } else {
            m.pieces.clone()
        };
        let dir = destination.join(&m.id);
        private_dir(&dir)?;
        let mut moved = Vec::new();
        for (n, p) in pieces.iter().enumerate() {
            ensure!(
                !cancel.load(Ordering::Relaxed),
                "Relocation stopped; original weights are intact"
            );
            let to = dir.join(p.path.file_name().context("Model filename")?);
            ensure!(
                fs2::available_space(&dir)? > p.bytes.saturating_add(64 * 1024 * 1024),
                "Not enough space for cache relocation"
            );
            if !to.exists() {
                let tmp = to.with_extension("relocating");
                tokio::fs::copy(&p.path, &tmp).await?;
                let (size, hash) = hash_file(&tmp, &cancel, |_| {}).await?;
                ensure!(
                    size == p.bytes && hash == p.sha256,
                    "Relocated copy failed verification"
                );
                tokio::fs::rename(tmp, &to).await?;
            } else {
                let (size, hash) = hash_file(&to, &cancel, |_| {}).await?;
                ensure!(
                    size == p.bytes && hash == p.sha256,
                    "Cache destination conflicts with another file"
                );
            }
            if n == 0 {
                m.path = to.clone();
            }
            moved.push(LocalPart {
                path: to,
                bytes: p.bytes,
                sha256: p.sha256.clone(),
            });
        }
        m.pieces = moved;
        save_artifact(root, &m)?;
        if !journal["completed"]
            .as_array()
            .context("Relocation journal")?
            .iter()
            .any(|v| v == &m.id)
        {
            journal["completed"]
                .as_array_mut()
                .context("Relocation journal")?
                .push(serde_json::json!(m.id));
        }
        atomic_write(&journal_path, &serde_json::to_vec_pretty(&journal)?)?;
    }
    atomic_write(
        &root.join("model-cache.json"),
        &serde_json::to_vec(&destination)?,
    )?;
    let receipt = serde_json::json!({"cache":destination,"previous_cache":journal["previous_cache"],"originals_retained":journal["originals"],"note":"Imported originals are untouched. Verified copies are active. Review retained old paths before reclaiming space."});
    atomic_write(
        &root.join("cache-relocation.json"),
        &serde_json::to_vec_pretty(&receipt)?,
    )?;
    std::fs::remove_file(journal_path)?;
    std::fs::File::open(root)?.sync_all()?;
    Ok(receipt)
}

/// Download a pinned artifact. Resume validates Content-Range and the complete
/// file digest; only verified complete bytes are atomically promoted to `path`.
pub async fn download_verified(
    url: &str,
    path: &Path,
    size: u64,
    expected: &str,
    cancel: Cancel,
    mut progress: impl FnMut(Progress),
) -> Result<()> {
    ensure!(
        size > 0 && expected.len() == 64 && expected.chars().all(|c| c.is_ascii_hexdigit()),
        "Artifact has no usable size/checksum"
    );
    let parent = path.parent().context("Invalid download destination")?;
    private_dir(parent)?;
    let lock_path = path.with_extension("lock");
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)?;
    fs2::FileExt::try_lock_exclusive(&lock)
        .context("This file is already being downloaded in another Alt window")?;
    if path.exists() {
        let (bytes, hash) = hash_file(path, &cancel, &mut progress).await?;
        ensure!(
            bytes == size && hash.eq_ignore_ascii_case(expected),
            "An existing file failed its checksum. Move it aside before downloading again"
        );
        return Ok(());
    }
    let partial = path.with_extension("partial");
    let mut offset = tokio::fs::metadata(&partial)
        .await
        .map(|m| m.len())
        .unwrap_or(0);
    ensure!(
        offset <= size,
        "The partial download is larger than expected; remove the .partial file and retry"
    );
    let available = fs2::available_space(parent)?;
    ensure!(
        available >= size.saturating_sub(offset).saturating_add(64 * 1024 * 1024),
        "Not enough free disk space. This download still needs {}",
        human_bytes(size.saturating_sub(offset))
    );
    if offset < size {
        let mut request = hub_request(client()?.get(url), url)?
            .header(reqwest::header::ACCEPT_ENCODING, "identity");
        if offset > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={offset}-"));
        }
        let response = tokio::select! {
            _=cancelled(&cancel)=>bail!("Download paused. Select the same file to resume"),
            result=tokio::time::timeout(Duration::from_secs(30),request.send())=>result,
        }
        .context("The download server did not respond. Retry to resume")?
        .context(
            "Cannot reach the file host. Check internet access; retry will resume the download",
        )?
        .error_for_status()
        .context("The file host refused the download. Check network access and retry")?;
        if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
            let range = response
                .headers()
                .get(reqwest::header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                .context("Resume response did not include Content-Range")?;
            let expected_range = format!("bytes {offset}-{}", size - 1);
            ensure!(
                range == format!("{expected_range}/{size}"),
                "The server returned an unexpected byte range; the partial file was preserved"
            );
        } else if response.status() == reqwest::StatusCode::OK {
            offset = 0;
        } else {
            bail!("Unexpected download response: {}", response.status());
        }
        let mut output = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(offset == 0)
            .append(offset > 0)
            .open(&partial)
            .await?;
        let mut stream = response.bytes_stream();
        let mut last = Instant::now();
        let transfer: Result<()> = async {
            loop {
                ensure!(
                    !cancel.load(Ordering::Relaxed),
                    "Download paused. Select the same file to resume"
                );
                let next = tokio::select! {
                    _=cancelled(&cancel)=>bail!("Download paused. Select the same file to resume"),
                    result=tokio::time::timeout(Duration::from_secs(30),stream.next())=>result,
                }
                .context("Download stalled. Select this file again to resume")?;
                let Some(chunk) = next else {
                    break;
                };
                let chunk =
                    chunk.context("Download interrupted. Select this file again to resume")?;
                ensure!(
                    offset + chunk.len() as u64 <= size,
                    "The file host sent more data than expected"
                );
                output.write_all(&chunk).await.context(
                    "Could not write the download. Check free disk space and permissions",
                )?;
                offset += chunk.len() as u64;
                if last.elapsed() > Duration::from_millis(150) {
                    progress(Progress {
                        received: offset,
                        total: size,
                        stage: "Downloading".into(),
                    });
                    last = Instant::now();
                }
            }
            Ok(())
        }
        .await;
        // Tokio may still have a blocking write in flight when a stream fails.
        // Finish it before reporting a resumable offset or allowing another attempt.
        output
            .flush()
            .await
            .context("Could not flush partial download; check disk space before retrying")?;
        output.sync_all().await?;
        transfer?;
        ensure!(
            offset == size,
            "The file is incomplete. Select it again to resume"
        );
    }
    let (bytes, hash) = hash_file(&partial, &cancel, &mut progress).await?;
    if bytes != size || !hash.eq_ignore_ascii_case(expected) {
        tokio::fs::rename(&partial, path.with_extension("corrupt")).await?;
        bail!(
            "Checksum mismatch. The file was quarantined as .corrupt and will not be used; retry for a fresh download"
        );
    }
    tokio::fs::rename(&partial, path).await?;
    std::fs::File::open(parent)?.sync_all()?;
    progress(Progress {
        received: size,
        total: size,
        stage: "Verified and ready".into(),
    });
    Ok(())
}

pub fn human_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GiB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else {
        format!("{:.0} MiB", bytes as f64 / (1024.0 * 1024.0))
    }
}
