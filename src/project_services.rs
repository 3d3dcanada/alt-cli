//! Project verification plans, incremental retrieval and visible context.
use crate::project::{CheckResult, Project, digest, excluded};
use anyhow::{Result, ensure};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub name: String,
    pub description: String,
    pub check_name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementStatus {
    pub requirement: Requirement,
    pub status: String,
    pub evidence: Option<String>,
    pub check_kind: crate::verification::Kind,
    pub coverage: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verification {
    pub complete: bool,
    pub behavioral_acceptance: bool,
    pub scope: String,
    pub requirements: Vec<RequirementStatus>,
    pub source_snapshot: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexReport {
    pub indexed: usize,
    pub updated: usize,
    pub removed: usize,
    pub skipped: usize,
    pub method: String,
    #[serde(default)]
    pub elapsed_ms: u64,
    #[serde(default)]
    pub source_bytes_read: u64,
    #[serde(default)]
    pub sampled_process_rss_bytes: Option<u64>,
}

pub fn environment(cwd: &Path, argv: &[String]) -> BTreeMap<String, String> {
    environment_cancellable(cwd, argv, None)
}
pub fn environment_cancellable(
    cwd: &Path,
    argv: &[String],
    cancel: Option<&AtomicBool>,
) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    // Old records counted only saved output and can miss a trailing zero-test
    // summary. A verifier correction must require new evidence after upgrade.
    result.insert("verification_contract".into(), "3".into());
    match crate::verification::dependencies_cancellable(cwd, cancel) {
        Ok(hash) => {
            result.insert("installed_dependencies".into(), hash);
        }
        Err(error) => {
            result.insert(
                "unavailable_dependency_inventory".into(),
                format!("{error:#}"),
            );
        }
    }
    result.insert(
        "argv_sha256".into(),
        digest(serde_json::to_string(argv).unwrap_or_default().as_bytes()),
    );
    for name in [
        "Cargo.lock",
        "package-lock.json",
        "yarn.lock",
        "pnpm-lock.yaml",
        "uv.lock",
        "poetry.lock",
        "requirements.txt",
        "rust-toolchain.toml",
        "rust-toolchain",
        ".python-version",
        ".tool-versions",
    ] {
        if let Ok(bytes) = std::fs::read(cwd.join(name)) {
            result.insert(name.into(), digest(&bytes));
        }
    }
    for name in [
        "PATH",
        "PYTHONPATH",
        "VIRTUAL_ENV",
        "NODE_ENV",
        "CARGO_HOME",
        "RUSTUP_HOME",
        "RUSTUP_TOOLCHAIN",
    ] {
        if let Some(value) = std::env::var_os(name) {
            result.insert(
                format!("environment_{name}_sha256"),
                digest(value.as_encoded_bytes()),
            );
        }
    }
    if let Some(program) = argv.first() {
        let paths = std::env::var_os("PATH").unwrap_or_default();
        let path = if program.contains('/') {
            Some(cwd.join(program))
        } else {
            std::env::split_paths(&paths)
                .map(|p| p.join(program))
                .find(|p| p.is_file())
        };
        if let Some(path) = path.and_then(|p| p.canonicalize().ok()) {
            result.insert("executable".into(), path.display().to_string());
            if let Ok(m) = path.metadata() {
                result.insert(
                    "executable_metadata".into(),
                    format!("{}:{:?}", m.len(), m.modified()),
                );
            }
        }
    }
    result
}

pub fn environment_current(check: &CheckResult, cwd: &Path, argv: &[String]) -> bool {
    environment_current_cancellable(check, cwd, argv, None)
}
pub fn environment_current_cancellable(
    check: &CheckResult,
    cwd: &Path,
    argv: &[String],
    cancel: Option<&AtomicBool>,
) -> bool {
    if check
        .environment
        .keys()
        .any(|k| k.starts_with("unavailable_"))
    {
        return false;
    }
    check
        .environment
        .iter()
        .filter(|(key, _)| !key.starts_with("observed_version_"))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<BTreeMap<_, _>>()
        == environment_cancellable(cwd, argv, cancel)
}
/// Recognize explicit test-run summaries; unknown command output remains unknown.
pub fn test_count(output: &str) -> Option<u64> {
    let mut counts = TestCounts::default();
    counts.push(output.as_bytes());
    counts.finish().total()
}
/// Scan runner summaries independently of the bounded saved output. Keep at
/// most one 16 KiB line and counters, even when a command emits gigabytes.
#[derive(Default)]
pub(crate) struct TestCounts {
    line: Vec<u8>,
    oversized: bool,
    counts: [Option<u64>; 4],
    no_tests: bool,
}
impl TestCounts {
    pub(crate) fn push(&mut self, bytes: &[u8]) {
        for part in bytes.split_inclusive(|b| *b == b'\n') {
            if !self.oversized && self.line.len() + part.len() <= 16 * 1024 {
                self.line.extend_from_slice(part);
            } else {
                self.line.clear();
                self.oversized = true;
            }
            if part.ends_with(b"\n") {
                self.record_line();
            }
        }
    }
    fn record_line(&mut self) {
        static PATTERNS: std::sync::LazyLock<[regex::Regex; 4]> = std::sync::LazyLock::new(|| {
            [
                r"test result: .*? (\d+) passed; (\d+) failed",
                r"^Ran (\d+) tests?",
                r"^# tests (\d+)",
                r"(\d+) passed(?:,| in |\s|$)",
            ]
            .map(|p| regex::Regex::new(p).expect("constant regex"))
        });
        if !self.oversized {
            let line = String::from_utf8_lossy(&self.line);
            for (i, pattern) in PATTERNS.iter().enumerate() {
                for capture in pattern.captures_iter(&line) {
                    if let Ok(mut count) = capture[1].parse::<u64>() {
                        if i == 0 {
                            let Ok(failed) = capture[2].parse::<u64>() else {
                                continue;
                            };
                            count = count.saturating_add(failed);
                        }
                        self.counts[i] = Some(self.counts[i].unwrap_or(0).saturating_add(count));
                    }
                }
            }
            self.no_tests |= line.contains("no tests ran");
        }
        self.line.clear();
        self.oversized = false;
    }
    pub(crate) fn finish(mut self) -> Self {
        self.record_line();
        self
    }
    pub(crate) fn merge(mut self, other: Self) -> Self {
        for (a, b) in self.counts.iter_mut().zip(other.counts) {
            if let Some(b) = b {
                *a = Some(a.unwrap_or(0).saturating_add(b));
            }
        }
        self.no_tests |= other.no_tests;
        self
    }
    pub(crate) fn total(&self) -> Option<u64> {
        self.counts
            .iter()
            .copied()
            .flatten()
            .next()
            .or_else(|| self.no_tests.then_some(0))
    }
}
impl Project {
    /// Bounded structural neighbors supplement lexical localization without extra models.
    pub fn structural_context(&mut self, query: &str) -> Result<Vec<serde_json::Value>> {
        let query = &query[..query.floor_char_boundary(1000)];
        if query.trim().is_empty() {
            return Ok(vec![]);
        }
        let hits = self.search(query)?;
        let paths = self.browse()?;
        let mut result = Vec::new();
        let mut selected = BTreeSet::new();
        for hit in hits.into_iter().take(4) {
            let Some(path) = hit["path"].as_str() else {
                continue;
            };
            if !selected.insert(path.to_owned()) {
                continue;
            }
            if let Some(bytes) = self.bytes(path)?
                && let Ok(body) = String::from_utf8(bytes)
            {
                let hash = digest(body.as_bytes());
                let syntax = crate::syntax::chunks(path, &body)?;
                let outline:Vec<_>=syntax.chunks.iter().take(8).map(|s|serde_json::json!({"name":s.name,"kind":s.kind,"lines":[s.start_line,s.end_line]})).collect();
                let mut neighbors = Vec::new();
                for dep in syntax.dependencies.iter().take(8) {
                    if let Some(module) = &dep.module {
                        let stem = module.trim_start_matches("./").replace('.', "/");
                        for other in paths
                            .iter()
                            .filter(|other| {
                                other.as_str() != path
                                    && (other.ends_with(&format!("{stem}.py"))
                                        || other.ends_with(&format!("{stem}/__init__.py"))
                                        || other.ends_with(module.trim_start_matches("./")))
                            })
                            .take(2)
                        {
                            if let Some(bytes) = self.bytes(other)? {
                                neighbors.push(serde_json::json!({"path":other,"sha256":digest(&bytes),"relation":"local import candidate; lexical resolution, inspect to confirm"}));
                            }
                        }
                    }
                }
                neighbors.truncate(4);
                result.push(serde_json::json!({"path":path,"sha256":hash,"outline":outline,"dependencies":syntax.dependencies.into_iter().take(8).collect::<Vec<_>>(),"neighbors":neighbors,"provenance":"Current source bytes; syntax outline and bounded local import candidates. Read before editing."}));
            }
        }
        Ok(result)
    }
    pub fn set_requirement(&self, r: &Requirement) -> Result<()> {
        ensure!(
            !r.name.trim().is_empty() && r.name.len() <= 120 && r.description.len() <= 4000,
            "Give the requirement a name up to 120 bytes and description up to 4 KB"
        );
        ensure!(
            self.checks()?.iter().any(|c| c.name == r.check_name),
            "Configure the named check first"
        );
        self.db.execute(
            "INSERT OR REPLACE INTO verification_requirements VALUES(?1,?2,?3)",
            params![r.name, r.description, r.check_name],
        )?;
        Ok(())
    }
    pub fn remove_requirement(&self, name: &str) -> Result<()> {
        self.db.execute(
            "DELETE FROM verification_requirements WHERE name=?1",
            [name],
        )?;
        Ok(())
    }
    pub fn requirements(&self) -> Result<Vec<Requirement>> {
        let mut s = self.db.prepare(
            "SELECT name,description,check_name FROM verification_requirements ORDER BY name",
        )?;
        Ok(s.query_map([], |r| {
            Ok(Requirement {
                name: r.get(0)?,
                description: r.get(1)?,
                check_name: r.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
    }
    pub fn verification(&self) -> Result<Verification> {
        let specs = self.checks()?;
        let required = self.requirements()?;
        let snapshot = if required.is_empty() {
            None
        } else {
            self.snapshot().ok().map(|s| s.0)
        };
        let mut statuses = Vec::new();
        for requirement in required {
            let raw:Option<String>=self.db.query_row("SELECT payload FROM checks WHERE json_extract(payload,'$.name')=?1 ORDER BY rowid DESC LIMIT 1",[&requirement.check_name],|r|r.get(0)).optional()?;
            let check = raw
                .map(|s| serde_json::from_str::<CheckResult>(&s))
                .transpose()?;
            let spec = specs.iter().find(|s| s.name == requirement.check_name);
            let status = match (&check, spec) {
                (_, None) => "check missing",
                (None, _) => "not run",
                (Some(c), Some(s))
                    if snapshot.as_ref() != Some(&c.snapshot)
                        || c.argv != s.argv
                        || c.contract != s.contract =>
                {
                    "stale source or command"
                }
                (Some(c), Some(s)) if !environment_current(c, &self.root, &s.argv) => {
                    "stale environment"
                }
                (_, Some(s)) if s.contract.assertion_bytes().is_err() => {
                    "stale independent assertion"
                }
                (Some(c), _)
                    if c.tests_run == Some(0)
                        && matches!(
                            c.contract.kind,
                            crate::verification::Kind::Tests | crate::verification::Kind::Custom
                        ) =>
                {
                    "zero tests"
                }
                (Some(c), _) if c.error.is_some() => "could not complete",
                (Some(c), _) if c.cancelled => "cancelled",
                (Some(c), _) if c.timed_out => "timed out",
                (Some(c), _) if c.exit_code != Some(0) => "failed",
                (Some(c), _)
                    if c.contract.kind == crate::verification::Kind::Tests
                        && c.structured
                            .as_ref()
                            .is_none_or(|r| !r.complete || r.failed > 0 || r.executed == 0) =>
                {
                    "test evidence incomplete"
                }
                _ => "passed on current files",
            };
            statuses.push(RequirementStatus {
                requirement,
                status: status.into(),
                check_kind: spec.map(|s| s.contract.kind).unwrap_or_default(),
                coverage: if check
                    .as_ref()
                    .and_then(|c| c.structured.as_ref())
                    .is_some_and(|r| r.assertion_verified)
                {
                    "independent behavioral assertion"
                } else if spec.is_some_and(|s| s.contract.kind == crate::verification::Kind::Tests)
                {
                    "runner test report; no independent assertion"
                } else {
                    "command result; behavioral coverage unknown"
                }
                .into(),
                evidence: check.map(|c| c.id),
            });
        }
        let complete = !statuses.is_empty()
            && statuses
                .iter()
                .all(|s| s.status == "passed on current files");
        let behavioral_acceptance = complete
            && statuses
                .iter()
                .all(|s| s.coverage == "independent behavioral assertion");
        Ok(Verification {
            complete,
            behavioral_acceptance,
            scope:if behavioral_acceptance{"Pinned independent assertions passed on current files; review their behavioral coverage"}else if complete{"Required commands passed; independent behavioral acceptance is not established"}else{"Required checks are incomplete"}.into(),
            requirements: statuses,
            source_snapshot: snapshot,
        })
    }
    pub fn record_token_usage(
        &self,
        task: &str,
        memory: &str,
        tokens: Option<usize>,
        budget: usize,
    ) -> Result<()> {
        let mut context = self.context_view(task)?;
        context["memory"] = serde_json::json!(memory);
        context["characters"] = serde_json::json!(memory.chars().count());
        context["memory_tokens"] = serde_json::json!(tokens);
        context["memory_token_budget"] = serde_json::json!(budget);
        context["token_accounting"] = serde_json::json!(if tokens.is_some() {
            "Selected llama.cpp tokenizer, memory block only; tool/chat/template tokens are separately engine-budgeted"
        } else {
            "Truncated using measured tokenizer ratio; final count unavailable"
        });
        self.db.execute(
            "INSERT OR REPLACE INTO context_views VALUES(?1,?2)",
            params![task, context.to_string()],
        )?;
        Ok(())
    }
    pub fn pin(&self, body: &str) -> Result<String> {
        ensure!(
            !body.trim().is_empty() && body.len() <= 4000,
            "Enter a requirement up to 4 KB"
        );
        let id = uuid::Uuid::new_v4().to_string();
        self.db.execute(
            "INSERT INTO pinned_requirements(id,body) VALUES(?1,?2)",
            params![id, body],
        )?;
        Ok(id)
    }
    pub fn unpin(&self, id: &str) -> Result<()> {
        self.db
            .execute("DELETE FROM pinned_requirements WHERE id=?1", [id])?;
        Ok(())
    }
    pub fn pinned(&self) -> Result<Vec<serde_json::Value>> {
        let mut s = self
            .db
            .prepare("SELECT id,body FROM pinned_requirements ORDER BY created,id")?;
        Ok(s.query_map([], |r| {
            Ok(serde_json::json!({"id":r.get::<_,String>(0)?,"requirement":r.get::<_,String>(1)?}))
        })?
        .collect::<rusqlite::Result<_>>()?)
    }
    pub fn context_view(&self, task: &str) -> Result<serde_json::Value> {
        let raw: Option<String> = self
            .db
            .query_row(
                "SELECT payload FROM context_views WHERE task=?1",
                [task],
                |r| r.get(0),
            )
            .optional()?;
        Ok(raw.map(|s|serde_json::from_str(&s)).transpose()?.unwrap_or(serde_json::json!({"notice":"No model context has been assembled for this task yet","pinned":self.pinned()?})))
    }
    /// Metadata scans avoid watcher overflow and missed rename events. Only changed
    /// files are read/tokenized. ctime + inode detect same-size/mtime replacements.
    pub fn browse(&self) -> Result<Vec<String>> {
        self.browse_cancellable(self.cancellation.as_deref())
    }
    fn browse_cancellable(&self, cancel: Option<&AtomicBool>) -> Result<Vec<String>> {
        fn walk(
            p: &Project,
            base: &str,
            depth: usize,
            out: &mut Vec<String>,
            cancel: Option<&AtomicBool>,
        ) -> Result<()> {
            ensure!(
                depth < 64,
                "Index exceeds 50,000 files or 64 directory levels; select a narrower folder"
            );
            ensure!(
                !cancel.is_some_and(|c| c.load(Ordering::Relaxed)),
                "Indexing cancelled; previous index preserved"
            );
            let dir = if base.is_empty() {
                p.dir.try_clone()?
            } else {
                p.dir.open_dir(base)?
            };
            for entry in dir.entries()? {
                ensure!(
                    !cancel.is_some_and(|c| c.load(Ordering::Relaxed)),
                    "Indexing cancelled; previous index preserved"
                );
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().into_owned();
                if excluded(&name) {
                    continue;
                }
                let path = if base.is_empty() {
                    name
                } else {
                    format!("{base}/{name}")
                };
                let kind = entry.file_type()?;
                if kind.is_symlink() {
                    continue;
                }
                if kind.is_dir() {
                    walk(p, &path, depth + 1, out, cancel)?;
                } else if kind.is_file() {
                    ensure!(
                        out.len() < 50_000,
                        "Index exceeds 50,000 files; select a narrower folder"
                    );
                    out.push(path);
                }
            }
            Ok(())
        }
        let mut paths = Vec::new();
        walk(self, "", 0, &mut paths, cancel)?;
        paths.sort();
        Ok(paths)
    }
    pub fn index_incremental(&mut self, cancel: Option<&AtomicBool>) -> Result<IndexReport> {
        let started = std::time::Instant::now();
        let paths = self.browse_cancellable(cancel)?;
        let mut seen = BTreeSet::new();
        let mut report = IndexReport {
            method:
                "Incremental metadata scan; changed content only; inode/ctime/mtime/size detection"
                    .into(),
            ..Default::default()
        };
        ensure!(
            fs2::available_space(&self.state)? > 64 * 1024 * 1024,
            "Not enough free space to update the search cache; previous index preserved"
        );
        let mut indexed_bytes = 0u64;
        let tx = self.db.transaction()?;
        for path in paths {
            ensure!(
                !cancel.is_some_and(|c| c.load(Ordering::Relaxed)),
                "Indexing cancelled; previous index preserved"
            );
            let metadata = self.dir.metadata(&path)?;
            if metadata.len() > 4 * 1024 * 1024
                || indexed_bytes.saturating_add(metadata.len()) > 256 * 1024 * 1024
            {
                report.skipped += 1;
                continue;
            }
            indexed_bytes += metadata.len();
            #[cfg(unix)]
            let signature = {
                use cap_std::fs::MetadataExt;
                format!(
                    "{}:{}:{}:{}:{}:{}",
                    metadata.ino(),
                    metadata.len(),
                    metadata.mtime(),
                    metadata.mtime_nsec(),
                    metadata.ctime(),
                    metadata.ctime_nsec()
                )
            };
            #[cfg(not(unix))]
            let signature = format!("{}:{:?}", metadata.len(), metadata.modified());
            seen.insert(path.clone());
            let old: Option<String> = tx
                .query_row(
                    "SELECT signature FROM index_meta WHERE path=?1",
                    [&path],
                    |r| r.get(0),
                )
                .optional()?;
            if old.as_ref() == Some(&signature) {
                report.indexed += 1;
                continue;
            }
            let mut bytes = Vec::new();
            self.dir
                .open(&path)?
                .take(4 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)?;
            report.source_bytes_read = report.source_bytes_read.saturating_add(bytes.len() as u64);
            let body = if bytes.len() <= 4 * 1024 * 1024 {
                String::from_utf8(bytes).ok()
            } else {
                None
            };
            tx.execute("DELETE FROM symbols WHERE path=?1", [&path])?;
            tx.execute("DELETE FROM chunk_search WHERE path=?1", [&path])?;
            tx.execute("DELETE FROM file_search WHERE path=?1", [&path])?;
            tx.execute("DELETE FROM files WHERE path=?1", [&path])?;
            tx.execute("DELETE FROM index_meta WHERE path=?1", [&path])?;
            if let Some(body) = body {
                let syntax = crate::syntax::chunks(&path, &body)?;
                tx.execute(
                    "INSERT INTO symbols VALUES(?1,?2)",
                    params![path, serde_json::to_string(&syntax)?],
                )?;
                for chunk in &syntax.chunks {
                    tx.execute("INSERT INTO chunk_search(path,start_line,end_line,body) VALUES(?1,?2,?3,?4)",params![path,chunk.start_line,chunk.end_line,&body[chunk.start_byte..chunk.end_byte]])?;
                }
                tx.execute(
                    "INSERT INTO files VALUES(?1,?2,?3)",
                    params![path, digest(body.as_bytes()), body],
                )?;
                tx.execute(
                    "INSERT INTO file_search(path,body) VALUES(?1,?2)",
                    params![path, body],
                )?;
                tx.execute(
                    "INSERT INTO index_meta VALUES(?1,?2)",
                    params![path, signature],
                )?;
                report.indexed += 1;
                report.updated += 1;
            } else {
                report.skipped += 1;
            }
        }
        let old: Vec<String> = tx
            .prepare("SELECT path FROM files")?
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        for path in old {
            if !seen.contains(&path) {
                tx.execute("DELETE FROM symbols WHERE path=?1", [&path])?;
                tx.execute("DELETE FROM chunk_search WHERE path=?1", [&path])?;
                tx.execute("DELETE FROM file_search WHERE path=?1", [&path])?;
                tx.execute("DELETE FROM files WHERE path=?1", [&path])?;
                tx.execute("DELETE FROM index_meta WHERE path=?1", [&path])?;
                report.removed += 1;
            }
        }
        tx.commit()?;
        report.elapsed_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
        report.sampled_process_rss_bytes = std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines().find_map(|line| {
                    line.strip_prefix("VmRSS:")
                        .and_then(|s| s.split_whitespace().next()?.parse::<u64>().ok())
                        .map(|n| n * 1024)
                })
            });
        Ok(report)
    }
    pub fn repository_map(&mut self) -> Result<serde_json::Value> {
        let index = self.index_incremental(None)?;
        let mut stmt = self
            .db
            .prepare("SELECT path,payload FROM symbols ORDER BY path LIMIT 2000")?;
        let mut files = Vec::new();
        for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
            let (path, payload) = row?;
            files.push(serde_json::json!({"path":path,"syntax":serde_json::from_str::<serde_json::Value>(&payload)?}));
        }
        Ok(
            serde_json::json!({"index":index,"files":files,"symbol_method":"Tree-sitter Rust/Python/JavaScript declarations and chunks; unsupported languages use full-text search"}),
        )
    }
}
