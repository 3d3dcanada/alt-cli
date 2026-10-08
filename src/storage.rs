//! Consistent SQLite snapshots and state archives. Model weights and live jobs are excluded.
use anyhow::{Context, Result, ensure};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};
#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    format: u32,
    alt_version: String,
    files: BTreeMap<String, String>,
    #[serde(default)]
    excluded: Vec<String>,
    #[serde(default)]
    generation: Option<String>,
}

/// Every coordinated state writer takes this shared barrier. Backups take its
/// exclusive side while freezing databases, configuration and execution receipts.
/// Never hold a Store lease for the lifetime of an idle database handle.
#[derive(Debug)]
pub struct StateWriteGuard(std::fs::File);
impl StateWriteGuard {
    pub fn acquire(root: &Path) -> Result<Self> {
        let file = generation_file(root)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
        loop {
            match fs2::FileExt::try_lock_shared(&file) {
                Ok(()) => return Ok(Self(file)),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock && std::time::Instant::now() < deadline => std::thread::sleep(std::time::Duration::from_millis(10)),
                Err(error) => return Err(error).context("A backup is freezing state; this write was not applied. Keep the unsaved input and retry after backup finishes"),
            }
        }
    }
}
impl Drop for StateWriteGuard {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self.0);
    }
}
fn generation_file(root: &Path) -> Result<std::fs::File> {
    crate::config::private_dir(root)?;
    let path = root.join("state-generation.lock");
    ensure!(!path.is_symlink(), "State generation lock cannot be a link");
    Ok(std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?)
}
pub fn write_guard_for_path(path: &Path) -> Result<Option<StateWriteGuard>> {
    for parent in path.ancestors().skip(1) {
        if parent.join("state-generation.lock").is_file() {
            return StateWriteGuard::acquire(parent).map(Some);
        }
    }
    Ok(None)
}
fn snapshot_guard(root: &Path) -> Result<std::fs::File> {
    let file = generation_file(root)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        match fs2::FileExt::try_lock_exclusive(&file) {
            Ok(()) => return Ok(file),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock && std::time::Instant::now() < deadline => std::thread::sleep(std::time::Duration::from_millis(20)),
            Err(error) => return Err(error).context("State is changing; finish or stop active work and retry the consistent backup. No archive was published"),
        }
    }
}
fn file_digest(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
const ARCHIVE_MAX_BYTES: u64 = 64 * 1024 * 1024 * 1024;
const ARCHIVE_MAX_FILES: usize = 1_000_000;
fn inference_payload(path: &Path) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    path.components()
        .next()
        .is_some_and(|p| p.as_os_str() == "inference")
        && (name.ends_with("-request.json")
            || name.ends_with("-response.raw")
            || name.starts_with("rejected-"))
}
fn paths(
    root: &Path,
    base: &Path,
    out: &mut Vec<PathBuf>,
    leases: &mut Vec<std::fs::File>,
) -> Result<()> {
    for entry in std::fs::read_dir(root.join(base))? {
        let e = entry?;
        let name = e.file_name();
        let text = name.to_string_lossy();
        let rel = base.join(&name);
        let kind = e.file_type()?;
        if text == "active.lock" && base.components().any(|p| p.as_os_str() == "execution") {
            let lease = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(e.path())?;
            fs2::FileExt::try_lock_exclusive(&lease).context("An execution receipt is still being recorded; stop or finish that check and retry backup")?;
            leases.push(lease);
        }
        if kind.is_symlink()
            || matches!(
                text.as_ref(),
                "jobs" | "blobs" | "tools" | "engine-v3" | "downloads" | "inference-archives"
            )
            || text.ends_with("-wal")
            || text.ends_with("-shm")
            || text.ends_with(".lock")
            || text.ends_with(".sock")
            || text.ends_with(".gguf")
            || text.ends_with(".partial")
            || text.ends_with(".corrupt")
        {
            continue;
        }
        if kind.is_dir() {
            paths(root, &rel, out, leases)?;
        } else if kind.is_file() && !inference_payload(&rel) {
            out.push(rel);
        }
        ensure!(
            out.len() <= ARCHIVE_MAX_FILES,
            "State archive has too many files"
        );
    }
    Ok(())
}
pub fn backup(root: &Path, destination: &Path) -> Result<serde_json::Value> {
    let generation = snapshot_guard(root)?;
    backup_snapshot(root, destination, generation)
}
fn backup_snapshot(
    root: &Path,
    destination: &Path,
    generation: std::fs::File,
) -> Result<serde_json::Value> {
    let destination = if destination.is_absolute() {
        destination.into()
    } else {
        std::env::current_dir()?.join(destination)
    };
    ensure!(
        !destination.starts_with(root),
        "Save backups outside the active data folder to avoid recursive archives"
    );
    ensure!(
        !destination.exists(),
        "Backup destination already exists; choose a new file"
    );
    let parent = destination.parent().context("Choose a backup folder")?;
    std::fs::create_dir_all(parent)?;
    ensure!(
        !parent.canonicalize()?.starts_with(root.canonicalize()?),
        "Save backups outside the active data folder, including symbolic links"
    );
    let staging = tempfile::tempdir()?;
    let mut files = Vec::new();
    let mut execution_leases = Vec::new();
    paths(root, Path::new(""), &mut files, &mut execution_leases)?;
    files.sort();
    let mut manifest = Manifest {
        format: 1,
        alt_version: env!("CARGO_PKG_VERSION").into(),
        files: BTreeMap::new(),
        excluded: vec!["Model weights, installed tools and live jobs".into(), "Raw inference requests/responses and inference-archives; use state export-inference for those payloads. Effective settings, cost/integrity receipts and archive indexes remain in this backup".into()],
        generation: Some(uuid::Uuid::new_v4().to_string()),
    };
    let mut total = 0u64;
    for rel in &files {
        let source = root.join(rel);
        let dest = staging.path().join(rel);
        std::fs::create_dir_all(dest.parent().unwrap())?;
        ensure!(
            fs2::available_space(staging.path())?
                >= source.metadata()?.len().saturating_add(16 * 1024 * 1024),
            "Not enough free space to freeze this state file; no backup was published"
        );
        if source
            .extension()
            .is_some_and(|x| x == "db" || x == "sqlite")
        {
            let db = rusqlite::Connection::open_with_flags(
                &source,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )?;
            db.busy_timeout(std::time::Duration::from_secs(5))?;
            db.execute("VACUUM INTO ?1", [dest.to_string_lossy().as_ref()])?;
        } else {
            std::fs::copy(&source, &dest)?;
        }
        total += dest.metadata()?.len();
        ensure!(
            total <= ARCHIVE_MAX_BYTES,
            "Recovery state exceeds 64 GiB; export old inference sessions and review storage before backing up"
        );
        manifest
            .files
            .insert(rel.to_string_lossy().into(), file_digest(&dest)?);
    }
    std::fs::write(
        staging.path().join("ALT-BACKUP.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    // All protected files and database images are now frozen; compression does
    // not need to pause writers.
    drop(execution_leases);
    drop(generation);
    let temp = tempfile::NamedTempFile::new_in(parent)?;
    let encoder = GzEncoder::new(temp, Compression::default());
    let mut archive = tar::Builder::new(encoder);
    archive.append_dir_all("state", staging.path())?;
    let encoder = archive.into_inner()?;
    let mut temp = encoder.finish()?;
    temp.flush()?;
    temp.as_file().sync_all()?;
    temp.persist_noclobber(&destination).map_err(|e| e.error)?;
    let sha = file_digest(&destination)?;
    std::fs::File::open(parent)?.sync_all()?;
    Ok(
        serde_json::json!({"path":destination,"sha256":sha,"files":files.len(),"uncompressed_bytes":total,"excluded":manifest.excluded,"generation":manifest.generation,"consistency":"Exclusive coordinated state generation barrier; all SQLite files frozen while state mutations are blocked. Raw archival inference payloads are exported separately."}),
    )
}
pub fn restore(archive: &Path, destination: &Path) -> Result<serde_json::Value> {
    ensure!(
        !destination.exists() || std::fs::read_dir(destination)?.next().is_none(),
        "Restore into a new or empty data directory; existing state is never overwritten"
    );
    let parent = destination
        .parent()
        .context("Choose a destination folder")?;
    std::fs::create_dir_all(parent)?;
    let staging = tempfile::tempdir_in(parent)?;
    let mut tar = tar::Archive::new(GzDecoder::new(std::fs::File::open(archive)?));
    let mut total = 0u64;
    let mut count = 0;
    let mut extracted = std::collections::BTreeSet::new();
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        ensure!(
            path.components().all(|c| matches!(c, Component::Normal(_)))
                && path.starts_with("state"),
            "Invalid path in state archive"
        );
        ensure!(
            entry.header().entry_type().is_file() || entry.header().entry_type().is_dir(),
            "State archives cannot contain links or special files"
        );
        if entry.header().entry_type().is_file() {
            ensure!(
                extracted.insert(path.strip_prefix("state")?.to_string_lossy().to_string()),
                "Duplicate file in archive"
            );
        }
        total += entry.size();
        count += 1;
        ensure!(
            total <= ARCHIVE_MAX_BYTES && count <= 2 * ARCHIVE_MAX_FILES + 1,
            "State archive exceeds restore limits"
        );
        ensure!(
            fs2::available_space(staging.path())? >= entry.size().saturating_add(16 * 1024 * 1024),
            "Not enough free space for restored state; existing state is unchanged"
        );
        ensure!(
            entry.unpack_in(staging.path())?,
            "Archive path escaped the restore folder"
        );
    }
    let state = staging.path().join("state");
    ensure!(
        state.join("ALT-BACKUP.json").metadata()?.len() <= 64 * 1024 * 1024,
        "Backup manifest exceeds 64 MiB"
    );
    let manifest: Manifest =
        serde_json::from_slice(&std::fs::read(state.join("ALT-BACKUP.json"))?)?;
    ensure!(
        matches!(manifest.format, 1 | 2),
        "Unsupported backup format"
    );
    for (rel, expected) in &manifest.files {
        ensure!(
            Path::new(rel)
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
            "Invalid manifest path"
        );
        ensure!(
            file_digest(&state.join(rel))? == *expected,
            "Backup integrity failed for {rel}"
        );
    }
    extracted.remove("ALT-BACKUP.json");
    ensure!(
        extracted == manifest.files.keys().cloned().collect(),
        "Archive includes files outside its manifest"
    );
    std::fs::remove_file(state.join("ALT-BACKUP.json"))?;
    crate::config::private_dir(&state)?;
    std::fs::rename(&state, destination)?;
    std::fs::File::open(parent)?.sync_all()?;
    Ok(
        serde_json::json!({"restored_to":destination,"files":manifest.files.len(),"backup_version":manifest.alt_version,"generation":manifest.generation,"excluded":manifest.excluded,"note":"Project source and model weights were not restored or modified. Verify imported model paths on this machine."}),
    )
}
/// A visible storage warning threshold; it never deletes data or cuts inference.
pub fn set_budget(root: &Path, max_inference_bytes: u64) -> Result<serde_json::Value> {
    ensure!(
        (1024 * 1024..=1024 * 1024 * 1024 * 1024).contains(&max_inference_bytes),
        "Choose an inference evidence warning budget from 1 MiB to 1 TiB"
    );
    let _generation = StateWriteGuard::acquire(root)?;
    let value = serde_json::json!({"schema":1,"inference_warning_bytes":max_inference_bytes,"enforcement":"Warning only; explicit archival retention preserves receipts and active work"});
    crate::config::atomic_write(
        &root.join("storage-budget.json"),
        &serde_json::to_vec_pretty(&value)?,
    )?;
    Ok(value)
}
fn inference_budget(root: &Path) -> Result<u64> {
    let path = root.join("storage-budget.json");
    if !path.exists() {
        return Ok(512 * 1024 * 1024);
    }
    ensure!(
        path.metadata()?.len() <= 4096,
        "Storage budget file exceeds 4 KiB"
    );
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    value["inference_warning_bytes"]
        .as_u64()
        .context("Invalid storage warning budget")
}
pub fn usage(root: &Path) -> Result<serde_json::Value> {
    fn size(path: &Path) -> u64 {
        let Ok(m) = std::fs::symlink_metadata(path) else {
            return 0;
        };
        if m.is_symlink() {
            0
        } else if m.is_file() {
            m.len()
        } else {
            std::fs::read_dir(path)
                .map(|d| d.flatten().map(|e| size(&e.path())).sum())
                .unwrap_or(0)
        }
    }
    let mut values = BTreeMap::new();
    for entry in std::fs::read_dir(root)? {
        let e = entry?;
        values.insert(e.file_name().to_string_lossy().to_string(), size(&e.path()));
    }
    let budget = inference_budget(root)?;
    let inference_bytes = values.get("inference").copied().unwrap_or(0);
    Ok(
        serde_json::json!({"total_bytes":values.values().sum::<u64>(),"categories":values,"inference_warning_bytes":budget,"inference_over_budget":inference_bytes > budget,"action":if inference_bytes > budget { "Preview Storage retention, then export inactive inference payloads. Keep or move the verified archives; receipts and active connections remain protected." } else { "Within the configured inference evidence warning budget" },"backup_scope":"Core state excludes raw inference payloads and inference archives; export those separately. Their receipts and archive indexes remain included."}),
    )
}
pub fn diagnostics(root: &Path) -> Result<serde_json::Value> {
    let config = crate::config::Config::load(root)?;
    let profiles:Vec<_>=config.profiles.iter().map(|(name,p)|serde_json::json!({"name":name,"provider":p.provider,"model":p.model,"context":p.context_tokens,"authentication_configured":p.api_key_env.is_some(),"local_model":p.local_model.is_some()})).collect();
    Ok(
        serde_json::json!({"alt_version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"architecture":std::env::consts::ARCH,"profiles":profiles,"storage":usage(root)?,"omitted":"Credentials, environment values, endpoints, project paths, prompts, source files and raw logs. Review before sharing."}),
    )
}
/// Export exact inference evidence before retention. An advisory lease prevents
/// exporting a live connection. The archive is written atomically, with hashes.
pub fn export_inference(
    root: &Path,
    connection: &str,
    destination: &Path,
) -> Result<serde_json::Value> {
    ensure!(
        !connection.is_empty()
            && Path::new(connection).components().count() == 1
            && Path::new(connection)
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
        "Choose a single inference connection ID"
    );
    let _generation = StateWriteGuard::acquire(root)?;
    let directory = root.join("inference").join(connection);
    ensure!(
        directory.is_dir() && !directory.is_symlink(),
        "Inference connection does not exist"
    );
    let lease = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join("active.lock"))?;
    fs2::FileExt::try_lock_exclusive(&lease)
        .context("This inference connection is active; disconnect it before exporting")?;
    let destination = if destination.is_absolute() {
        destination.to_path_buf()
    } else {
        std::env::current_dir()?.join(destination)
    };
    export_inference_locked(&directory, &destination)
}
fn export_inference_locked(directory: &Path, destination: &Path) -> Result<serde_json::Value> {
    ensure!(
        !destination.exists(),
        "Inference export already exists; choose a new archive"
    );
    let parent = destination
        .parent()
        .context("Choose an inference archive folder")?;
    std::fs::create_dir_all(parent)?;
    ensure!(
        !parent
            .canonicalize()?
            .starts_with(directory.canonicalize()?),
        "Save inference archives outside their source connection"
    );
    let mut files = BTreeMap::new();
    let mut bytes = 0u64;
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".lock") {
            continue;
        }
        ensure!(
            entry.file_type()?.is_file(),
            "Inference export refuses linked or nested evidence entries"
        );
        bytes = bytes
            .checked_add(entry.metadata()?.len())
            .context("Evidence size overflow")?;
        ensure!(
            bytes <= ARCHIVE_MAX_BYTES && files.len() < ARCHIVE_MAX_FILES,
            "Inference export exceeds 64 GiB / one million files"
        );
        files.insert(name, file_digest(&entry.path())?);
    }
    let temp = tempfile::NamedTempFile::new_in(parent)?;
    let mut archive = tar::Builder::new(GzEncoder::new(temp, Compression::default()));
    for name in files.keys() {
        archive.append_path_with_name(directory.join(name), Path::new("inference").join(name))?;
    }
    let manifest = serde_json::to_vec_pretty(
        &serde_json::json!({"format":1,"connection":directory.file_name(),"files":files,"uncompressed_bytes":bytes}),
    )?;
    let mut header = tar::Header::new_gnu();
    header.set_size(manifest.len() as u64);
    header.set_mode(0o600);
    header.set_cksum();
    archive.append_data(&mut header, "INFERENCE-MANIFEST.json", &manifest[..])?;
    let mut temp = archive.into_inner()?.finish()?;
    temp.flush()?;
    temp.as_file().sync_all()?;
    for (name, hash) in &files {
        ensure!(
            file_digest(&directory.join(name))? == *hash,
            "Inference evidence changed while exporting; no archive published"
        );
    }
    temp.persist_noclobber(destination).map_err(|e| e.error)?;
    std::fs::File::open(parent)?.sync_all()?;
    Ok(
        serde_json::json!({"archive":destination,"sha256":file_digest(destination)?,"files":files,"uncompressed_bytes":bytes}),
    )
}
fn inference_retention(
    root: &Path,
    age: std::time::Duration,
    apply: bool,
) -> Result<serde_json::Value> {
    let directory = root.join("inference");
    let mut eligible = Vec::new();
    let mut protected = Vec::new();
    let mut reclaimed = 0u64;
    if !directory.exists() {
        return Ok(
            serde_json::json!({"connections":eligible,"protected":protected,"reclaimable_bytes":0}),
        );
    }
    let _generation = StateWriteGuard::acquire(root)?;
    for entry in std::fs::read_dir(&directory)? {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        if !path.join("active.lock").is_file() {
            protected.push(serde_json::json!({"connection":id,"reason":"Legacy connection has no activity lease; explicitly export it after stopping old Alt processes"}));
            continue;
        }
        let lease = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path.join("active.lock"))?;
        if fs2::FileExt::try_lock_exclusive(&lease).is_err() {
            protected.push(serde_json::json!({"connection":id,"reason":"Active connection"}));
            continue;
        }
        let mut payloads = Vec::new();
        let mut size = 0u64;
        let mut recent = false;
        for item in std::fs::read_dir(&path)? {
            let item = item?;
            if !item.file_type()?.is_file() {
                continue;
            }
            let metadata = item.metadata()?;
            if item.file_name() != "active.lock"
                && metadata.modified()?.elapsed().unwrap_or_default() <= age
            {
                recent = true;
            }
            if inference_payload(&Path::new("inference").join(&id).join(item.file_name())) {
                size += metadata.len();
                payloads.push(item.path());
            }
        }
        if recent || payloads.is_empty() {
            continue;
        }
        reclaimed += size;
        let mut row = serde_json::json!({"connection":id,"payload_bytes":size,"payload_files":payloads.len(),"applied":false});
        if apply {
            let archive_dir = root.join("inference-archives");
            crate::config::private_dir(&archive_dir)?;
            let export = export_inference_locked(
                &path,
                &archive_dir.join(format!("{}-{}.tar.gz", id, uuid::Uuid::new_v4())),
            )?;
            crate::config::atomic_write(
                &path.join("archived.json"),
                &serde_json::to_vec_pretty(&export)?,
            )?;
            for payload in payloads {
                std::fs::remove_file(payload)?;
            }
            std::fs::File::open(&path)?.sync_all()?;
            row["applied"] = serde_json::json!(true);
            row["export"] = export;
        }
        eligible.push(row);
    }
    Ok(
        serde_json::json!({"connections":eligible,"protected":protected,"reclaimable_bytes":reclaimed,"policy":"Export before removing request/response payloads; active connections and cost/integrity receipts are retained. Inference archives are kept separately from core state backups."}),
    )
}
pub fn retain(root: &Path, days: u64, apply: bool) -> Result<serde_json::Value> {
    let age = std::time::Duration::from_secs(days.saturating_mul(86400));
    ensure!(days >= 1, "Keep at least one day of history");
    let inference = inference_retention(root, age, apply)?;
    let _generation = StateWriteGuard::acquire(root)?;
    let mut remove = Vec::new();
    for category in ["exports", "evaluations"] {
        let dir = root.join(category);
        if !dir.exists() {
            continue;
        }
        for e in std::fs::read_dir(dir)? {
            let e = e?;
            let m = e.metadata()?;
            if e.file_type()?.is_file() && m.modified()?.elapsed().unwrap_or_default() > age {
                remove.push(e.path());
            }
        }
    }
    for r in crate::jobs::list(root)? {
        if !matches!(r.status.as_str(), "running" | "starting" | "interrupted")
            && std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs()
                .saturating_sub(r.started)
                > age.as_secs()
        {
            remove.push(root.join("jobs").join(r.id));
        }
    }
    if apply {
        for path in &remove {
            let m = std::fs::symlink_metadata(path)?;
            ensure!(!m.is_symlink(), "Retention target changed to a link");
            if m.is_dir() {
                std::fs::remove_dir_all(path)?;
            } else {
                std::fs::remove_file(path)?;
            }
        }
    }
    Ok(
        serde_json::json!({"applied":apply,"keep_days":days,"targets":remove,"inference":inference,"preserved":"Conversation databases, project checkpoints, migrations, model files and imported originals"}),
    )
}
