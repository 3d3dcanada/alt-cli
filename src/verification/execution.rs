//! Durable execution receipts and bounded raw logs, independent of project-lock availability.
use super::*;
use crate::project::{CheckResult, Project, digest};
use fs2::FileExt;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Seek, SeekFrom, Write},
};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

const RECEIPT_LIMIT: usize = 2 * 1024 * 1024;
const LOG_LIMIT: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionEvidence {
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub snapshot_root: Option<PathBuf>,
    #[serde(default)]
    pub namespace_root: Option<PathBuf>,
    #[serde(default)]
    pub project_root: Option<PathBuf>,
    #[serde(default)]
    pub source_scope: String,
    #[serde(default)]
    pub before_manifest_sha256: Option<String>,
    #[serde(default)]
    pub after_manifest_sha256: Option<String>,
    #[serde(default)]
    pub changed_inputs: Vec<String>,
    #[serde(default)]
    pub patch_preview_truncated: bool,
    #[serde(default)]
    pub transient_or_metadata_changes: Vec<String>,
    #[serde(default)]
    pub generated_paths: Vec<String>,
    #[serde(default)]
    pub observed_inputs_stable: bool,
    #[serde(default)]
    pub immutable_inputs: bool,
    #[serde(default)]
    pub receipt: Option<String>,
    #[serde(default)]
    pub stdout: Option<RawLog>,
    #[serde(default)]
    pub stderr: Option<RawLog>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawLog {
    pub total_bytes: u64,
    pub retained_bytes: u64,
    pub ring_start: u64,
    pub sha256: String,
    pub scope: String,
}

pub struct LogWriter {
    file: tokio::fs::File,
    path: PathBuf,
    total: u64,
}
impl LogWriter {
    fn new(path: PathBuf) -> Result<Self> {
        let file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .read(true)
            .open(&path)?;
        Ok(Self {
            file: tokio::fs::File::from_std(file),
            path,
            total: 0,
        })
    }
    pub async fn append(&mut self, mut bytes: &[u8]) -> Result<()> {
        while !bytes.is_empty() {
            let at = self.total % LOG_LIMIT;
            let n = bytes.len().min((LOG_LIMIT - at) as usize);
            self.file.seek(SeekFrom::Start(at)).await?;
            self.file.write_all(&bytes[..n]).await?;
            self.total = self.total.saturating_add(n as u64);
            bytes = &bytes[n..];
        }
        Ok(())
    }
    pub async fn finish(mut self) -> Result<RawLog> {
        self.file.flush().await?;
        self.file.sync_all().await?;
        let retained = self.total.min(LOG_LIMIT);
        let start = if self.total > LOG_LIMIT {
            self.total % LOG_LIMIT
        } else {
            0
        };
        let mut bytes = std::fs::read(&self.path)?;
        ensure!(
            bytes.len() as u64 == retained,
            "Raw log length changed during capture"
        );
        bytes.rotate_left(start as usize);
        Ok(RawLog { total_bytes: self.total, retained_bytes: retained, ring_start: start, sha256: digest(&bytes), scope: "Last 4 MiB of this stream; offsets count all observed bytes. Earlier bytes may have rotated out.".into() })
    }
}

#[derive(Serialize, Deserialize)]
struct Receipt {
    schema: u32,
    id: String,
    task: String,
    #[serde(default)]
    check: Option<CheckResult>,
    #[serde(default)]
    terminal: Option<crate::sandbox::TerminalResult>,
}

struct UnstartedSpool {
    directory: PathBuf,
    pending: PathBuf,
    armed: bool,
}
impl Drop for UnstartedSpool {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(&self.pending);
            let _ = std::fs::remove_dir_all(&self.directory);
        }
    }
}

pub struct ExecutionSpool {
    pub directory: PathBuf,
    pending: PathBuf,
    lock: File,
    reserve: File,
    id: String,
    task: String,
    state_root: PathBuf,
}
impl Drop for ExecutionSpool {
    fn drop(&mut self) {
        // No subprocess can start until started.json has been durably written.
        // Reclaim an unused reservation if initialization ran out of space.
        if !self.directory.join("started.json").exists()
            && !self.directory.join("receipt.json").exists()
        {
            let _ = std::fs::remove_file(&self.pending);
            let _ = std::fs::remove_dir_all(&self.directory);
        }
    }
}
impl ExecutionSpool {
    pub fn new(state: &Path, id: &str, task: &str) -> Result<Self> {
        uuid::Uuid::parse_str(id).context("Invalid execution identifier")?;
        let state_root = state
            .parent()
            .and_then(Path::parent)
            .context("Project state location")?
            .to_path_buf();
        let _generation = crate::storage::StateWriteGuard::acquire(&state_root)?;
        let directory = state.join("execution").join(id);
        crate::config::private_dir(&state.join("execution"))?;
        std::fs::create_dir(&directory)
            .context("Execution identifier already exists; no command was started")?;
        let pending = state.join("execution-pending").join(id);
        let mut cleanup = UnstartedSpool {
            directory: directory.clone(),
            pending: pending.clone(),
            armed: true,
        };
        crate::config::private_dir(&directory)?;
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join("active.lock"))?;
        FileExt::lock_shared(&lock)?;
        let mut reserve = std::fs::OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(directory.join("receipt.reserve"))?;
        // Allocate real blocks rather than a sparse set_len reservation.
        let block = vec![0u8; 64 * 1024];
        for _ in 0..RECEIPT_LIMIT / block.len() {
            reserve
                .write_all(&block)
                .context("Cannot reserve execution receipt capacity; no command was started")?;
        }
        reserve.sync_all()?;
        crate::config::atomic_write(&pending, b"pending\n")?;
        // Sync every newly introduced directory link, not only files inside it.
        // A completed reservation/start record must survive loss of the process
        // and be ordered before any command is allowed to run.
        File::open(&directory)?.sync_all()?;
        File::open(state.join("execution"))?.sync_all()?;
        File::open(state.join("execution-pending"))?.sync_all()?;
        File::open(state)?.sync_all()?;
        cleanup.armed = false;
        Ok(Self {
            directory,
            pending,
            lock,
            reserve,
            id: id.into(),
            task: task.into(),
            state_root,
        })
    }
    pub fn logs(&self) -> Result<(LogWriter, LogWriter)> {
        Ok((
            LogWriter::new(self.directory.join("stdout.log"))?,
            LogWriter::new(self.directory.join("stderr.log"))?,
        ))
    }
    pub fn started_check(&self, result: &CheckResult) -> Result<()> {
        crate::config::atomic_write(
            &self.directory.join("started.json"),
            &serde_json::to_vec(&Receipt {
                schema: 1,
                id: self.id.clone(),
                task: self.task.clone(),
                check: Some(result.clone()),
                terminal: None,
            })?,
        )
    }
    pub fn started_terminal(&self, result: &crate::sandbox::TerminalResult) -> Result<()> {
        crate::config::atomic_write(
            &self.directory.join("started.json"),
            &serde_json::to_vec(&Receipt {
                schema: 1,
                id: self.id.clone(),
                task: self.task.clone(),
                check: None,
                terminal: Some(result.clone()),
            })?,
        )
    }
    pub fn observations(&self, phase: &str, observed: &InputObservation) -> Result<String> {
        ensure!(
            matches!(phase, "before" | "after"),
            "Invalid observation phase"
        );
        let bytes = serde_json::to_vec(observed)?;
        ensure!(
            bytes.len() <= 16 * 1024 * 1024,
            "Source observation manifest exceeds 16 MiB"
        );
        crate::config::atomic_write(&self.directory.join(format!("source-{phase}.json")), &bytes)?;
        Ok(digest(&bytes))
    }
    pub fn report(&self, bytes: &[u8]) -> Result<()> {
        crate::config::atomic_write(&self.directory.join("report"), bytes)
    }
    pub fn patch(&self, bytes: &[u8]) -> Result<()> {
        crate::config::atomic_write(&self.directory.join("source.patch"), bytes)
    }
    pub fn finish_check(self, result: CheckResult) -> Result<()> {
        let receipt = Receipt {
            schema: 1,
            id: self.id.clone(),
            task: self.task.clone(),
            check: Some(result),
            terminal: None,
        };
        self.finish(receipt)
    }
    pub fn finish_terminal(self, result: crate::sandbox::TerminalResult) -> Result<()> {
        let receipt = Receipt {
            schema: 1,
            id: self.id.clone(),
            task: self.task.clone(),
            check: None,
            terminal: Some(result),
        };
        self.finish(receipt)
    }
    fn finish(mut self, receipt: Receipt) -> Result<()> {
        let mut reserved_complete = false;
        let operation = (|| -> Result<()> {
            let bytes = serde_json::to_vec(&receipt)?;
            ensure!(
                bytes.len() <= RECEIPT_LIMIT,
                "Execution receipt exceeds reserved capacity"
            );
            self.reserve.seek(SeekFrom::Start(0))?;
            self.reserve.write_all(&bytes)?;
            self.reserve.set_len(bytes.len() as u64)?;
            self.reserve.sync_all()?;
            reserved_complete = true;
            // Active spool leases already exclude this folder from a coherent
            // backup. Preserve the completion in its reserved inode before
            // waiting for the generation barrier needed to publish the name.
            let _generation = crate::storage::StateWriteGuard::acquire(&self.state_root)?;
            std::fs::rename(
                self.directory.join("receipt.reserve"),
                self.directory.join("receipt.json"),
            )?;
            File::open(&self.directory)?.sync_all()?;
            Ok(())
        })();
        FileExt::unlock(&self.lock)?;
        operation.with_context(|| if reserved_complete {
            format!("Completed execution receipt retained in {} (receipt.reserve or receipt.json); finalization requires recovery. Do not rerun automatically; preserve/export the execution folder.",self.directory.display())
        } else {
            format!("Execution finished but its final receipt could not be durably saved. Do not rerun automatically. Preserve/export recovery files at {} (pending {}).",self.directory.display(),self.pending.display())
        })
    }
}

/// Called while Project's exclusive lock is held. Never opens Project recursively.
pub fn reconcile_pending(project: &Project) -> Result<()> {
    let pending = project.state.join("execution-pending");
    if !pending.exists() {
        return Ok(());
    }
    let mut entries = std::fs::read_dir(&pending)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries.into_iter().take(256) {
        let id = entry.file_name().to_string_lossy().into_owned();
        if uuid::Uuid::parse_str(&id).is_err() {
            continue;
        }
        let directory = project.state.join("execution").join(&id);
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(directory.join("active.lock"))?;
        match lock.try_lock_exclusive() {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(e) => return Err(e.into()),
        }
        let path = directory.join("receipt.json");
        let reserved = read_receipt(&directory.join("receipt.reserve"))
            .ok()
            .filter(|r| r.schema == 1 && r.id == id && r.check.is_some() != r.terminal.is_some());
        let mut receipt: Receipt = if path.exists() {
            read_receipt(&path)?
        } else if let Some(reserved) = reserved {
            std::fs::rename(directory.join("receipt.reserve"), &path)?;
            File::open(&directory)?.sync_all()?;
            reserved
        } else if directory.join("started.json").exists() {
            let mut r = read_receipt(&directory.join("started.json"))?;
            let message = format!(
                "Execution interrupted before a durable completion receipt. It was not replayed. Preserve raw recovery files at {}",
                directory.display()
            );
            if let Some(c) = &mut r.check {
                c.error = Some(message.clone());
                c.cancelled = true;
            }
            if let Some(t) = &mut r.terminal {
                t.error = Some(message);
                t.cancelled = true;
            }
            let bytes = serde_json::to_vec(&r)?;
            ensure!(
                bytes.len() <= RECEIPT_LIMIT,
                "Interrupted receipt exceeds reservation"
            );
            let reserve_path = directory.join("receipt.reserve");
            if reserve_path.is_file() {
                let mut reserve = std::fs::OpenOptions::new()
                    .write(true)
                    .open(&reserve_path)?;
                reserve.seek(SeekFrom::Start(0))?;
                reserve.write_all(&bytes)?;
                reserve.set_len(bytes.len() as u64)?;
                reserve.sync_all()?;
                std::fs::rename(&reserve_path, &path)?;
                File::open(&directory)?.sync_all()?;
            } else {
                crate::config::atomic_write(&path, &bytes)?;
            }
            r
        } else {
            // Interrupted reservation/setup, before any execution was allowed.
            std::fs::remove_file(entry.path())?;
            std::fs::remove_dir_all(&directory)?;
            continue;
        };
        ensure!(
            receipt.schema == 1 && receipt.id == id,
            "Execution receipt identity mismatch"
        );
        if let Some(check) = receipt.check.take() {
            ensure!(check.id == id, "Check receipt identity mismatch");
            for (phase, expected) in [
                ("before", &check.execution.before_manifest_sha256),
                ("after", &check.execution.after_manifest_sha256),
            ] {
                if let Some(expected) = expected {
                    let bytes = read_bounded(
                        &directory.join(format!("source-{phase}.json")),
                        16 * 1024 * 1024,
                    )?;
                    ensure!(
                        digest(&bytes) == *expected,
                        "Source observation manifest integrity failed; retain the pending receipt for recovery"
                    );
                }
            }
            if let Some(report) = &check.structured {
                let bytes = read_bounded(&directory.join("report"), super::REPORT_LIMIT)?;
                ensure!(
                    digest(&bytes) == report.report_sha256,
                    "Spooled report integrity failed"
                );
                crate::config::atomic_write(
                    &project
                        .state
                        .join("check-reports")
                        .join(format!("{id}.report")),
                    &bytes,
                )?;
            }
            project.save_check(&check)?;
        }
        if let Some(terminal) = receipt.terminal.take() {
            ensure!(terminal.id == id, "Terminal receipt identity mismatch");
            let path = project.state.join(format!("terminal-{id}.json"));
            crate::config::atomic_write(&path, &serde_json::to_vec_pretty(&terminal)?)?;
            // One durable source ID makes reconciliation idempotent after a crash.
            let seen: bool = project.db.query_row(
                "SELECT EXISTS(SELECT 1 FROM notes WHERE task=?1 AND source=?2)",
                rusqlite::params![receipt.task, id],
                |r| r.get(0),
            )?;
            if !seen {
                project.note(&receipt.task, if terminal.exit_code==Some(0) && terminal.error.is_none(){"observation"}else{"failure"}, &format!("Full access terminal {id}: exit {:?}, timeout {}, cancelled {}. External effects are not undoable. Raw execution receipt: execution/{id}/receipt.json. Command: {}\n{}",terminal.exit_code,terminal.timed_out,terminal.cancelled,crate::project::bounded(terminal.execution.command.as_deref().unwrap_or("unknown"),2000),crate::project::bounded(&terminal.output,4000)), &id)?;
            }
        }
        std::fs::remove_file(entry.path())?;
        File::open(&pending)?.sync_all()?;
        FileExt::unlock(&lock)?;
    }
    Ok(())
}
fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= limit,
        "Execution evidence exceeds its size bound"
    );
    Ok(bytes)
}
fn read_receipt(path: &Path) -> Result<Receipt> {
    Ok(serde_json::from_slice(&read_bounded(path, RECEIPT_LIMIT)?)?)
}

pub fn read_execution_log(
    project: &Project,
    id: &str,
    stream: &str,
    offset: Option<u64>,
    limit: usize,
) -> Result<Value> {
    uuid::Uuid::parse_str(id).context("Invalid execution ID")?;
    ensure!(
        matches!(stream, "stdout" | "stderr"),
        "Stream must be stdout or stderr"
    );
    ensure!(
        (1..=65536).contains(&limit),
        "Choose a log range of 1–65536 bytes"
    );
    let directory = project.state.join("execution").join(id);
    let receipt = read_receipt(&directory.join("receipt.json"))?;
    let evidence = receipt
        .check
        .as_ref()
        .map(|c| &c.execution)
        .or_else(|| receipt.terminal.as_ref().map(|t| &t.execution))
        .context("Receipt lacks execution evidence")?;
    let log = if stream == "stdout" {
        &evidence.stdout
    } else {
        &evidence.stderr
    }
    .as_ref()
    .context("No complete raw-log index; inspect the retained recovery files")?;
    let mut bytes = read_bounded(&directory.join(format!("{stream}.log")), LOG_LIMIT as usize)?;
    ensure!(
        bytes.len() as u64 == log.retained_bytes
            && log.ring_start <= log.retained_bytes
            && log.total_bytes >= log.retained_bytes,
        "Raw log length/index mismatch"
    );
    bytes.rotate_left(log.ring_start as usize);
    ensure!(digest(&bytes) == log.sha256, "Raw log integrity failed");
    let first = log.total_bytes - log.retained_bytes;
    let requested = offset.unwrap_or(log.total_bytes.saturating_sub(limit as u64).max(first));
    ensure!(
        requested >= first && requested <= log.total_bytes,
        "Requested bytes are unavailable; retained offsets are {first}..{}",
        log.total_bytes
    );
    let start = (requested - first) as usize;
    let end = (start + limit).min(bytes.len());
    Ok(
        json!({"id":id,"stream":stream,"offset":requested,"next_offset":first+end as u64,"retained_from":first,"total_bytes":log.total_bytes,"sha256":log.sha256,"text":String::from_utf8_lossy(&bytes[start..end]),"scope":log.scope}),
    )
}

pub type InputObservation = BTreeMap<String, (String, String)>;
pub fn observe_inputs(
    root: &Path,
    paths: impl IntoIterator<Item = String>,
) -> Result<InputObservation> {
    let dir = cap_std::fs::Dir::open_ambient_dir(root, cap_std::ambient_authority())?;
    let mut result = BTreeMap::new();
    for path in paths {
        let metadata = dir.symlink_metadata(&path)?;
        ensure!(
            metadata.is_file() && !metadata.is_symlink(),
            "Input is no longer a regular file: {path}"
        );
        let mut bytes = Vec::new();
        dir.open(&path)?
            .take(crate::project::FILE_LIMIT + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= crate::project::FILE_LIMIT,
            "Input grew beyond source limit: {path}"
        );
        #[cfg(unix)]
        let identity = {
            use cap_std::fs::MetadataExt;
            format!(
                "{}:{}:{}:{}:{}:{}:{}",
                metadata.dev(),
                metadata.ino(),
                metadata.mode(),
                metadata.ctime(),
                metadata.ctime_nsec(),
                metadata.mtime(),
                metadata.mtime_nsec()
            )
        };
        #[cfg(not(unix))]
        let identity = format!(
            "{:?}:{:?}:{}",
            metadata.created(),
            metadata.modified(),
            metadata.len()
        );
        result.insert(path, (digest(&bytes), identity));
    }
    Ok(result)
}

/// Inventory new source paths without following dependency or generated-output links.
pub fn new_execution_paths(
    root: &Path,
    original: &std::collections::BTreeSet<String>,
    report: Option<&str>,
    generated: &[String],
) -> Result<(Vec<String>, Vec<String>)> {
    fn walk(
        dir: &cap_std::fs::Dir,
        base: &str,
        depth: usize,
        count: &mut usize,
        out: &mut Vec<String>,
    ) -> Result<()> {
        ensure!(
            depth <= 32 && *count <= 8192,
            "Execution output inventory exceeds bounded scan"
        );
        for entry in dir.entries()? {
            let entry = entry?;
            *count += 1;
            ensure!(
                *count <= 8192,
                "Execution output inventory exceeds bounded scan"
            );
            let name = entry
                .file_name()
                .to_str()
                .context("Execution produced a non-UTF8 source path")?
                .to_owned();
            if crate::project::excluded(&name) {
                continue;
            }
            let path = if base.is_empty() {
                name
            } else {
                format!("{base}/{name}")
            };
            let kind = entry.file_type()?;
            if kind.is_dir() {
                walk(
                    &dir.open_dir(entry.file_name())?,
                    &path,
                    depth + 1,
                    count,
                    out,
                )?;
            } else {
                out.push(path);
            }
        }
        Ok(())
    }
    let dir = cap_std::fs::Dir::open_ambient_dir(root, cap_std::ambient_authority())?;
    let mut paths = Vec::new();
    walk(&dir, "", 0, &mut 0, &mut paths)?;
    let (mut declared, mut changed) = (Vec::new(), Vec::new());
    for path in paths {
        if original.contains(&path) {
            continue;
        }
        if report == Some(path.as_str())
            || generated
                .iter()
                .any(|g| Path::new(&path).starts_with(Path::new(g)))
        {
            declared.push(path);
        } else {
            changed.push(path);
        }
    }
    declared.sort();
    changed.sort();
    Ok((declared, changed))
}

/// A bounded textual review preview; binary/special-file changes are explicitly incomplete.
pub(crate) fn append_source_patch(
    root: &Path,
    path: &str,
    before: Option<&[u8]>,
    preview: &mut String,
) -> Result<bool> {
    crate::project::Project::validate_path(path)?;
    let dir = cap_std::fs::Dir::open_ambient_dir(root, cap_std::ambient_authority())?;
    let current = match dir.symlink_metadata(path) {
        Ok(metadata) => {
            ensure!(
                metadata.is_file() && !metadata.is_symlink(),
                "Changed input is a link or special file"
            );
            let mut bytes = Vec::new();
            dir.open(path)?
                .take(crate::project::FILE_LIMIT + 1)
                .read_to_end(&mut bytes)?;
            ensure!(
                bytes.len() as u64 <= crate::project::FILE_LIMIT,
                "Changed input exceeds preview limit"
            );
            Some(bytes)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    let old = std::str::from_utf8(before.unwrap_or(&[]))
        .context("Binary original has no textual patch preview")?;
    let new = std::str::from_utf8(current.as_deref().unwrap_or(&[]))
        .context("Binary result has no textual patch preview")?;
    let from = if before.is_some() {
        format!("a/{path}")
    } else {
        "/dev/null".into()
    };
    let to = if current.is_some() {
        format!("b/{path}")
    } else {
        "/dev/null".into()
    };
    let diff = similar::TextDiff::from_lines(old, new)
        .unified_diff()
        .header(&from, &to)
        .to_string();
    let remaining = (1024 * 1024usize).saturating_sub(preview.len());
    let end = diff.floor_char_boundary(remaining.min(diff.len()));
    preview.push_str(&diff[..end]);
    Ok(end == diff.len())
}
