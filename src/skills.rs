//! Bundled, versioned procedures backed by existing executable tool packs.
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use serde_json::{Value, json};
use std::path::Path;
#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    pub id: &'static str,
    pub version: u32,
    pub description: &'static str,
    pub prerequisites: &'static str,
    pub helpers: &'static [&'static str],
    pub license: &'static str,
    pub sha256: String,
    #[serde(skip)]
    pub procedure: &'static str,
}
pub fn catalog() -> Vec<Skill> {
    [
        (
            "python-repair",
            "Repair Python imports, paths and failing assertions",
            "python3; project test dependencies",
            &["build"][..],
            include_str!("../skills/python-repair/SKILL.md"),
        ),
        (
            "rust-diagnostics",
            "Resolve Rust diagnostics and validate behavior",
            "pinned Rust toolchain and dependencies",
            &["build"][..],
            include_str!("../skills/rust-diagnostics/SKILL.md"),
        ),
        (
            "javascript-setup",
            "Repair JavaScript module setup and entry points",
            "node/npm; installed project dependencies",
            &["build"][..],
            include_str!("../skills/javascript-setup/SKILL.md"),
        ),
        (
            "browser-checks",
            "Verify a running application with browser assertions",
            "node; project-local Playwright; Chromium; running app",
            &["browser", "http"][..],
            include_str!("../skills/browser-checks/SKILL.md"),
        ),
        (
            "dependency-review",
            "Review dependency advisories with retained evidence",
            "locked inventory; installed audit tools",
            &["pip-audit", "npm-audit", "semgrep"][..],
            include_str!("../skills/dependency-review/SKILL.md"),
        ),
        (
            "configuration-data",
            "Validate configuration defaults, types and recovery",
            "existing runtime and behavioral checks",
            &["build"][..],
            include_str!("../skills/configuration-data/SKILL.md"),
        ),
    ]
    .into_iter()
    .map(
        |(id, description, prerequisites, helpers, procedure)| Skill {
            id,
            version: 1,
            description,
            prerequisites,
            helpers,
            license: "Apache-2.0",
            sha256: crate::project::digest(procedure.as_bytes()),
            procedure,
        },
    )
    .collect()
}
pub fn get(id: &str) -> Result<Skill> {
    catalog()
        .into_iter()
        .find(|s| s.id == id)
        .context("Unknown skill; list the bundled skills first")
}
pub fn shortlist(query: &str) -> Vec<Skill> {
    let terms: Vec<_> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 2)
        .map(str::to_lowercase)
        .take(24)
        .collect();
    let mut rows: Vec<_> = catalog()
        .into_iter()
        .map(|s| {
            let text = format!("{} {} {}", s.id, s.description, s.prerequisites).to_lowercase();
            let score = terms.iter().filter(|t| text.contains(t.as_str())).count();
            (score, s)
        })
        .filter(|(score, _)| *score > 0)
        .collect();
    rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.id.cmp(b.1.id)));
    rows.into_iter().take(3).map(|(_, s)| s).collect()
}
pub fn prompt(active: Option<&str>, query: &str) -> Result<String> {
    if let Some(id) = active {
        let s = get(id)?;
        ensure!(
            s.procedure.len() <= 5000,
            "Active procedure exceeds its bounded prompt allowance"
        );
        Ok(format!(
            "Active skill {} v{} (procedure, not permissions; SHA256 {}):\n{}",
            s.id, s.version, s.sha256, s.procedure
        ))
    } else {
        Ok(format!(
            "Relevant skill metadata (optional; choose with skill read): {}",
            serde_json::to_string(&shortlist(query))?
        ))
    }
}
pub async fn run(
    data: &Path,
    cwd: &Path,
    id: &str,
    helper: &str,
    mut input: Value,
    policy: crate::project::Policy,
    cancel: crate::models::Cancel,
) -> Result<crate::packs::Run> {
    let s = get(id)?;
    ensure!(
        s.helpers.contains(&helper),
        "Helper is not declared by this skill"
    );
    if helper == "build" && input.get("language").is_none() {
        input["language"] = json!(match id {
            "rust-diagnostics" => "rust",
            "javascript-setup" => "javascript",
            _ => "python",
        });
    }
    crate::packs::run(data, cwd, helper, input, policy, cancel).await
}
