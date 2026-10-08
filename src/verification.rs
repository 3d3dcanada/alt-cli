//! Declared check purpose and bounded structured evidence, independent of model prose.
mod execution;
use anyhow::{Context, Result, bail, ensure};
pub use execution::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Tests,
    Build,
    Lint,
    Health,
    #[default]
    Custom,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Format {
    Json,
    Junit,
    Tap,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportSpec {
    pub format: Format,
    pub path: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assertion {
    pub path: PathBuf,
    pub sha256: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    #[serde(default)]
    pub inputs: ExecutionInputs,
    #[serde(default)]
    pub kind: Kind,
    pub report: Option<ReportSpec>,
    pub assertion: Option<Assertion>,
}
/// Explicit additional file dependencies and permitted newly generated outputs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionInputs {
    #[serde(default)]
    pub files: Vec<PathBuf>,
    #[serde(default)]
    pub generated: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub format: Format,
    pub passed: u64,
    pub failed: u64,
    pub skipped: u64,
    pub executed: u64,
    pub complete: bool,
    pub report_sha256: String,
    pub assertion_verified: bool,
}
impl Contract {
    pub fn validate(&self, cwd: &Path, argv: &[String]) -> Result<()> {
        ensure!(
            self.inputs.files.len() <= 64 && self.inputs.generated.len() <= 64,
            "Declare at most 64 external inputs and generated output paths"
        );
        for path in &self.inputs.files {
            let resolved = if path.is_absolute() {
                path.clone()
            } else {
                cwd.join(path)
            };
            ensure!(
                resolved.is_file(),
                "Declared input is unavailable: {}",
                resolved.display()
            );
        }
        for path in &self.inputs.generated {
            crate::project::Project::validate_path(path)?;
        }

        if self.kind == Kind::Tests {
            ensure!(
                self.report.is_some(),
                "Tests require a structured JSON, JUnit or TAP report; choose Custom for a command without a report"
            );
        }
        if let Some(report) = &self.report {
            crate::project::Project::validate_path(&report.path)?;
        }
        if let Some(assertion) = &self.assertion {
            ensure!(
                self.kind == Kind::Tests,
                "Independent assertions belong to a Tests check"
            );
            ensure!(
                !assertion
                    .path
                    .canonicalize()?
                    .starts_with(cwd.canonicalize()?),
                "Keep independent assertions outside the editable project"
            );
            let direct = argv.first().is_some_and(|a| a == "{assertion}");
            let interpreter = argv.get(1).is_some_and(|a| a == "{assertion}")
                && argv
                    .first()
                    .and_then(|a| Path::new(a).file_name())
                    .and_then(|a| a.to_str())
                    .is_some_and(|a| matches!(a, "python3" | "python" | "node" | "bash" | "sh"));
            ensure!(
                direct || interpreter,
                "Run {{assertion}} directly or as the first script argument to python3, python, node, bash or sh; embedding its name in an unrelated command does not prove execution"
            );
            ensure!(
                assertion.sha256.len() == 64
                    && assertion.sha256.bytes().all(|b| b.is_ascii_hexdigit()),
                "Invalid pinned assertion hash"
            );
        }
        for arg in argv {
            for placeholder in ["{assertion}", "{report}"] {
                ensure!(
                    !arg.contains(placeholder) || arg == placeholder,
                    "Use {placeholder} as a separate argument, not embedded in shell text"
                );
            }
        }
        ensure!(
            !argv.iter().any(|a| a == "{assertion}") || self.assertion.is_some(),
            "Pin an assertion file first"
        );
        ensure!(
            !argv.iter().any(|a| a == "{report}") || self.report.is_some(),
            "Choose a report path first"
        );
        Ok(())
    }
    pub fn assertion_bytes(&self) -> Result<Option<Vec<u8>>> {
        self.assertion.as_ref().map(|a| {
            let mut bytes=Vec::new();
            std::fs::File::open(&a.path)?.take(1024*1024+1).read_to_end(&mut bytes)?;
            ensure!(bytes.len()<=1024*1024,"Independent assertion exceeds 1 MiB");
            ensure!(crate::project::digest(&bytes)==a.sha256,"Independent assertion changed; review and explicitly pin it again before running");
            Ok(bytes)
        }).transpose()
    }
    pub fn pin(path: &Path) -> Result<Assertion> {
        let path = path.canonicalize()?;
        let mut bytes = Vec::new();
        std::fs::File::open(&path)?
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() <= 1024 * 1024,
            "Independent assertion exceeds 1 MiB"
        );
        Ok(Assertion {
            path,
            sha256: crate::project::digest(&bytes),
        })
    }
}
fn outcome(format: Format, bytes: &[u8], passed: u64, failed: u64, skipped: u64) -> Outcome {
    Outcome {
        format,
        passed,
        failed,
        skipped,
        executed: passed + failed,
        complete: true,
        report_sha256: crate::project::digest(bytes),
        assertion_verified: false,
    }
}
pub const REPORT_LIMIT: usize = 8 * 1024 * 1024;
pub fn parse(format: Format, bytes: &[u8]) -> Result<Outcome> {
    ensure!(
        bytes.len() <= REPORT_LIMIT,
        "Structured report exceeds 8 MiB; evidence is incomplete"
    );
    let text = std::str::from_utf8(bytes).context("Structured report is not UTF-8")?;
    match format {
        Format::Json => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Report {
                schema: u32,
                complete: bool,
                tests: Vec<Case>,
            }
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Case {
                name: String,
                status: String,
            }
            let r: Report = serde_json::from_slice(bytes)?;
            ensure!(
                r.schema == 1 && r.complete,
                "Test report is incomplete or has an unsupported schema"
            );
            ensure!(r.tests.len() <= 100_000, "Too many test records");
            let (mut passed, mut failed, mut skipped) = (0, 0, 0);
            let mut names = BTreeSet::new();
            for c in r.tests {
                ensure!(
                    !c.name.trim().is_empty() && names.insert(c.name),
                    "Missing or duplicate test identity"
                );
                match c.status.as_str() {
                    "passed" => passed += 1,
                    "failed" => failed += 1,
                    "skipped" => skipped += 1,
                    _ => bail!("Unknown or unfinished test status"),
                }
            }
            Ok(outcome(format, bytes, passed, failed, skipped))
        }
        Format::Junit => {
            ensure!(
                !text.contains("<!DOCTYPE") && !text.contains("<!ENTITY"),
                "JUnit report cannot declare entities or a DTD"
            );
            let doc = roxmltree::Document::parse_with_options(
                text,
                roxmltree::ParsingOptions {
                    nodes_limit: 400_000,
                    ..Default::default()
                },
            )?;
            let root = doc.root_element();
            ensure!(
                matches!(root.tag_name().name(), "testsuite" | "testsuites"),
                "Expected a JUnit testsuite/testsuites"
            );
            let (mut passed, mut failed, mut skipped) = (0, 0, 0);
            let mut ids = BTreeSet::new();
            for c in root.descendants().filter(|n| n.has_tag_name("testcase")) {
                let name = c.attribute("name").context("JUnit testcase name missing")?;
                let suite = c
                    .ancestors()
                    .filter_map(|n| {
                        if n.has_tag_name("testsuite") {
                            n.attribute("name")
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("/");
                ensure!(
                    !name.is_empty()
                        && ids.insert((suite, c.attribute("classname").unwrap_or(""), name)),
                    "Missing or duplicate JUnit test identity"
                );
                ensure!(ids.len() <= 100_000, "Too many test records");
                let bad = c
                    .children()
                    .any(|n| n.has_tag_name("failure") || n.has_tag_name("error"));
                let skip = c.children().any(|n| n.has_tag_name("skipped"));
                ensure!(!(bad && skip), "Contradictory JUnit testcase");
                if bad {
                    failed += 1
                } else if skip {
                    skipped += 1
                } else {
                    passed += 1
                }
            }
            for suite in root
                .descendants()
                .filter(|n| n.has_tag_name("testsuite") || n.has_tag_name("testsuites"))
            {
                if let Some(total) = suite.attribute("tests") {
                    ensure!(
                        total.parse::<usize>()?
                            == suite
                                .descendants()
                                .filter(|n| n.has_tag_name("testcase"))
                                .count(),
                        "JUnit test count does not match its complete testcase records"
                    );
                }
                for tag in ["failures", "errors", "skipped"] {
                    if let Some(count) = suite.attribute(tag) {
                        let child = match tag {
                            "failures" => "failure",
                            "errors" => "error",
                            _ => "skipped",
                        };
                        ensure!(
                            count.parse::<usize>()?
                                == suite
                                    .descendants()
                                    .filter(|n| n.has_tag_name(child))
                                    .count(),
                            "Contradictory JUnit {tag} count"
                        );
                    }
                }
            }
            Ok(outcome(format, bytes, passed, failed, skipped))
        }
        Format::Tap => {
            let (mut passed, mut failed, mut skipped) = (0, 0, 0);
            let mut plan = None;
            let mut ids = BTreeSet::new();
            let pattern = regex::Regex::new(r"^(not ok|ok)\s+(\d+)(?:\s|$)(.*)$")?;
            for raw in text.lines() {
                let line = raw.trim();
                if line.is_empty() || line == "TAP version 13" || line.starts_with('#') {
                    continue;
                }
                ensure!(!line.starts_with("Bail out!"), "TAP runner bailed out");
                ensure!(
                    raw == raw.trim_start(),
                    "Nested TAP requires a flattened report"
                );
                if let Some(rest) = line.strip_prefix("1..") {
                    ensure!(plan.is_none(), "Duplicate TAP plan");
                    let n = rest
                        .split_whitespace()
                        .next()
                        .context("Missing TAP plan")?
                        .parse::<u64>()?;
                    ensure!(n <= 100_000, "Too many TAP records");
                    plan = Some(n);
                    continue;
                }
                let c = pattern
                    .captures(line)
                    .context("Unsupported or incomplete TAP record")?;
                let id = c[2].parse::<u64>()?;
                ensure!(
                    id > 0 && ids.insert(id) && ids.len() <= 100_000,
                    "Invalid or duplicate TAP test number"
                );
                let directive = c[3]
                    .split_once('#')
                    .map(|(_, s)| s.trim().to_ascii_uppercase())
                    .unwrap_or_default();
                if directive.starts_with("SKIP") || directive.starts_with("TODO") {
                    skipped += 1
                } else if &c[1] == "not ok" {
                    failed += 1
                } else {
                    passed += 1
                }
            }
            let n = plan.context("TAP plan missing; evidence is incomplete")?;
            ensure!(
                ids.len() as u64 == n && ids.iter().copied().eq(1..=n),
                "TAP plan has missing or extra results"
            );
            Ok(outcome(format, bytes, passed, failed, skipped))
        }
    }
}
pub fn read_report(scratch: &Path, spec: &ReportSpec) -> Result<(Outcome, Vec<u8>)> {
    let dir = cap_std::fs::Dir::open_ambient_dir(scratch, cap_std::ambient_authority())?;
    let mut bytes = Vec::new();
    dir.open(&spec.path)?
        .take(REPORT_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)?;
    Ok((parse(spec.format, &bytes)?, bytes))
}

/// Metadata includes ctime and inode, so same-size/mtime in-place modifications
/// and replacements invalidate evidence without rehashing gigabytes of packages.
pub fn dependencies(cwd: &Path) -> Result<String> {
    dependencies_cancellable(cwd, None)
}
pub fn dependencies_cancellable(
    cwd: &Path,
    cancel: Option<&std::sync::atomic::AtomicBool>,
) -> Result<String> {
    use sha2::{Digest, Sha256};
    fn walk(
        path: &Path,
        h: &mut Sha256,
        count: &mut usize,
        depth: usize,
        seen: &mut std::collections::BTreeSet<std::path::PathBuf>,
        cancel: Option<&std::sync::atomic::AtomicBool>,
    ) -> Result<()> {
        ensure!(
            !cancel.is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed)),
            "Dependency inventory cancelled"
        );
        ensure!(
            depth < 64 && *count < 100_000,
            "Dependency inventory exceeds its bounded scan; choose a smaller environment"
        );
        let m = std::fs::symlink_metadata(path)?;
        *count += 1;
        h.update(path.as_os_str().as_encoded_bytes());
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            h.update(
                format!(
                    "{}:{}:{}:{}:{}:{}:{}",
                    m.ino(),
                    m.len(),
                    m.mode(),
                    m.mtime(),
                    m.mtime_nsec(),
                    m.ctime(),
                    m.ctime_nsec()
                )
                .as_bytes(),
            );
        }
        #[cfg(not(unix))]
        h.update(format!("{}:{:?}", m.len(), m.modified()).as_bytes());
        if m.is_symlink() {
            h.update(std::fs::read_link(path)?.as_os_str().as_encoded_bytes());
            let resolved = path.canonicalize()?;
            if seen.insert(resolved.clone()) {
                walk(&resolved, h, count, depth + 1, seen, cancel)?;
            }
        } else if m.is_dir() {
            let mut entries = std::fs::read_dir(path)?.collect::<std::io::Result<Vec<_>>>()?;
            entries.sort_by_key(|e| e.file_name());
            for e in entries {
                if !matches!(
                    e.file_name().to_str(),
                    Some("__pycache__" | ".cache" | ".git")
                ) {
                    walk(&e.path(), h, count, depth + 1, seen, cancel)?;
                }
            }
        }
        Ok(())
    }
    let mut h = Sha256::new();
    let mut count = 0;
    let mut seen = std::collections::BTreeSet::new();
    for name in ["node_modules", ".venv", "venv"] {
        let p = cwd.join(name);
        if p.symlink_metadata().is_ok() {
            walk(&p, &mut h, &mut count, 0, &mut seen, cancel)?;
        }
    }
    Ok(format!("{:x}", h.finalize()))
}
