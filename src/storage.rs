//! Consistent SQLite snapshots and state archives. Model weights and live jobs are excluded.
use anyhow::{Context, Result, ensure};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Component, Path, PathBuf},
};
#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    format: u32,
    alt_version: String,
    files: BTreeMap<String, String>,
}
fn paths(root: &Path, base: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(root.join(base))? {
        let e = entry?;
        let name = e.file_name();
        let text = name.to_string_lossy();
        let rel = base.join(&name);
        let kind = e.file_type()?;
        if kind.is_symlink()
            || matches!(
                text.as_ref(),
                "jobs" | "blobs" | "tools" | "engine-v3" | "downloads"
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
            paths(root, &rel, out)?;
        } else if kind.is_file() {
            out.push(rel);
        }
        ensure!(out.len() <= 50_000, "State archive has too many files");
    }
    Ok(())
}
pub fn backup(root: &Path, destination: &Path) -> Result<serde_json::Value> {
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
    paths(root, Path::new(""), &mut files)?;
    files.sort();
    let mut manifest = Manifest {
        format: 1,
        alt_version: env!("CARGO_PKG_VERSION").into(),
        files: BTreeMap::new(),
    };
    let mut total = 0u64;
    for rel in &files {
        let source = root.join(rel);
        let dest = staging.path().join(rel);
        std::fs::create_dir_all(dest.parent().unwrap())?;
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
            total <= 512 * 1024 * 1024,
            "State backup exceeds 512 MiB; review storage before backing up"
        );
        manifest.files.insert(
            rel.to_string_lossy().into(),
            crate::project::digest(&std::fs::read(&dest)?),
        );
    }
    std::fs::write(
        staging.path().join("ALT-BACKUP.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    let temp = tempfile::NamedTempFile::new_in(parent)?;
    let encoder = GzEncoder::new(temp, Compression::default());
    let mut archive = tar::Builder::new(encoder);
    archive.append_dir_all("state", staging.path())?;
    let encoder = archive.into_inner()?;
    let mut temp = encoder.finish()?;
    temp.flush()?;
    temp.as_file().sync_all()?;
    temp.persist_noclobber(&destination).map_err(|e| e.error)?;
    let sha = crate::project::digest(&std::fs::read(&destination)?);
    Ok(
        serde_json::json!({"path":destination,"sha256":sha,"files":files.len(),"uncompressed_bytes":total,"excluded":"Managed weights, tools, engine cache and live job records. Imported model paths remain references."}),
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
            total <= 512 * 1024 * 1024 && count <= 50_001,
            "State archive exceeds restore limits"
        );
        ensure!(
            entry.unpack_in(staging.path())?,
            "Archive path escaped the restore folder"
        );
    }
    let state = staging.path().join("state");
    let manifest: Manifest =
        serde_json::from_slice(&std::fs::read(state.join("ALT-BACKUP.json"))?)?;
    ensure!(manifest.format == 1, "Unsupported backup format");
    for (rel, expected) in &manifest.files {
        ensure!(
            Path::new(rel)
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
            "Invalid manifest path"
        );
        ensure!(
            crate::project::digest(&std::fs::read(state.join(rel))?) == *expected,
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
        serde_json::json!({"restored_to":destination,"files":manifest.files.len(),"backup_version":manifest.alt_version,"note":"Project source and model weights were not restored or modified. Verify imported model paths on this machine."}),
    )
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
    Ok(serde_json::json!({"total_bytes":values.values().sum::<u64>(),"categories":values}))
}
pub fn diagnostics(root: &Path) -> Result<serde_json::Value> {
    let config = crate::config::Config::load(root)?;
    let profiles:Vec<_>=config.profiles.iter().map(|(name,p)|serde_json::json!({"name":name,"provider":p.provider,"model":p.model,"context":p.context_tokens,"authentication_configured":p.api_key_env.is_some(),"local_model":p.local_model.is_some()})).collect();
    Ok(
        serde_json::json!({"alt_version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"architecture":std::env::consts::ARCH,"profiles":profiles,"storage":usage(root)?,"omitted":"Credentials, environment values, endpoints, project paths, prompts, source files and raw logs. Review before sharing."}),
    )
}
pub fn retain(root: &Path, days: u64, apply: bool) -> Result<serde_json::Value> {
    let age = std::time::Duration::from_secs(days.saturating_mul(86400));
    ensure!(days >= 1, "Keep at least one day of history");
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
        serde_json::json!({"applied":apply,"keep_days":days,"targets":remove,"preserved":"Conversation databases, project checkpoints, migrations, model files and imported originals"}),
    )
}
