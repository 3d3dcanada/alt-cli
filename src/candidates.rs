//! Serial source candidates. External effects retain the selected access mode.
//! Selection is independent verification; promotion is a separate explicit operation.
use crate::{
    config::{Config, Preferences, Profile},
    models::Cancel,
    project::{Policy, Project},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Effort {
    #[default]
    Quick,
    Careful,
    Thorough,
}
impl Effort {
    pub fn count(self) -> usize {
        match self {
            Self::Quick => 1,
            Self::Careful => 2,
            Self::Thorough => 3,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub id: String,
    pub source_root: PathBuf,
    pub baseline: String,
    pub goal: String,
    pub effort: Effort,
    pub seconds: u64,
    pub generated_tokens: u32,
    pub requests: u32,
    pub profile: Profile,
    pub preferences: Preferences,
    pub engine: PathBuf,
    pub engine_sha256: String,
    pub alt_sha256: String,
    pub checks: Vec<crate::project::CheckSpec>,
    pub requirements: Vec<crate::project_services::Requirement>,
    pub base_files: BTreeMap<String, String>,
    #[serde(default)]
    pub base_modes: BTreeMap<String, u32>,
    pub rows: Vec<Value>,
    pub status: String,
    pub spent_seconds: f64,
    pub spent_generated_tokens: u64,
    pub spent_requests: u32,
    pub selected: Option<usize>,
    #[serde(default)]
    pub in_flight: Option<Value>,
}
fn path(data: &Path, id: &str) -> Result<PathBuf> {
    uuid::Uuid::parse_str(id)?;
    Ok(data.join("candidates").join(id))
}
fn save(root: &Path, m: &Manifest) -> Result<()> {
    crate::config::atomic_write(&root.join("manifest.json"), &serde_json::to_vec_pretty(m)?)
}
pub fn load(data: &Path, id: &str) -> Result<Manifest> {
    Ok(serde_json::from_slice(&std::fs::read(
        path(data, id)?.join("manifest.json"),
    )?)?)
}
pub fn history(data: &Path) -> Result<Vec<Value>> {
    let root = data.join("candidates");
    if !root.exists() {
        return Ok(vec![]);
    }
    let mut rows = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let p = entry?.path().join("manifest.json");
        if p.is_file() {
            let m: Manifest = serde_json::from_slice(&std::fs::read(p)?)?;
            rows.push(json!({"id":m.id,"goal":m.goal,"status":m.status,"effort":m.effort,"selected":m.selected,"candidates":m.rows,"spent_seconds":m.spent_seconds,"spent_generated_tokens":m.spent_generated_tokens,"spent_requests":m.spent_requests}));
        }
    }
    Ok(rows)
}
#[allow(clippy::too_many_arguments)]
pub async fn run(
    data: &Path,
    cwd: &Path,
    engine: &Path,
    profile: &Profile,
    prefs: &Preferences,
    goal: &str,
    effort: Effort,
    seconds: u64,
    generated_tokens: u32,
    requests: u32,
    resume: Option<&str>,
    cancel: Cancel,
) -> Result<Manifest> {
    ensure!(
        profile.uncensored,
        "Candidate live runs require an explicit uncensored/abliterated model"
    );
    ensure!(
        (30..=3600).contains(&seconds)
            && generated_tokens >= 128
            && requests >= effort.count() as u32,
        "Use 30..3600 seconds, >=128 generated tokens and >=one request per candidate"
    );
    let engine = engine.canonicalize()?;
    let executable = std::env::current_exe()?;
    let (baseline, files, checks, requirements, modes) = {
        let p = Project::open(data, cwd)?;
        let (hash, files) = p.snapshot()?;
        let requirements = p.requirements()?;
        let checks = p.checks()?;
        ensure!(
            !requirements.is_empty()
                && requirements
                    .iter()
                    .all(|r| checks.iter().any(|s| s.name == r.check_name
                        && s.contract.kind == crate::verification::Kind::Tests
                        && s.contract.assertion.is_some())),
            "Candidate selection needs required independent Tests checks; configure them in Task first"
        );
        let modes = files
            .keys()
            .map(|name| Ok((name.clone(), p.mode(name)?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        (hash, files, checks, requirements, modes)
    };
    let mut m = if let Some(id) = resume {
        let m = load(data, id)?;
        ensure!(
            m.baseline == baseline
                && m.source_root == cwd.canonicalize()?
                && m.goal == goal
                && m.seconds == seconds
                && m.generated_tokens == generated_tokens
                && m.requests == requests
                && m.effort.count() == effort.count()
                && serde_json::to_value(&m.profile)? == serde_json::to_value(profile)?
                && serde_json::to_value(&m.preferences)? == serde_json::to_value(prefs)?
                && m.engine_sha256 == crate::capability::file_sha256(&engine)?
                && m.alt_sha256 == crate::capability::file_sha256(&executable)?
                && serde_json::to_value(&m.checks)? == serde_json::to_value(&checks)?
                && serde_json::to_value(&m.requirements)? == serde_json::to_value(&requirements)?,
            "Resume identity/source/checks changed; preserved candidates cannot be mixed with a new run"
        );
        m
    } else {
        Manifest {
            schema: 1,
            id: uuid::Uuid::new_v4().to_string(),
            source_root: cwd.canonicalize()?,
            baseline,
            goal: goal.into(),
            effort,
            seconds,
            generated_tokens,
            requests,
            profile: profile.clone(),
            preferences: prefs.clone(),
            engine: engine.clone(),
            engine_sha256: crate::capability::file_sha256(&engine)?,
            alt_sha256: crate::capability::file_sha256(&executable)?,
            checks,
            requirements,
            base_files: files
                .iter()
                .map(|(n, b)| (n.clone(), crate::project::digest(b)))
                .collect(),
            base_modes: modes.clone(),
            rows: vec![],
            status: "running".into(),
            spent_seconds: 0.0,
            spent_generated_tokens: 0,
            spent_requests: 0,
            selected: None,
            in_flight: None,
        }
    };
    let root = path(data, &m.id)?;
    crate::config::private_dir(&root)?;
    if let Some(interrupted) = m.in_flight.take() {
        // Reserved costs were persisted before launch. An interrupted request keeps its full allowance.
        m.rows.push(json!({"index":interrupted["index"],"eligible":false,"status":"interrupted; allowance conservatively charged","source_directory":interrupted["source_directory"],"state_directory":interrupted["state_directory"],"verification":null}));
        m.status = "running".into();
    }
    save(&root, &m)?;
    for index in m.rows.len()..effort.count() {
        if cancel.load(Ordering::Relaxed) {
            m.status = "cancelled".into();
            break;
        }
        let remaining = seconds as f64 - m.spent_seconds;
        let tokens = (generated_tokens as u64).saturating_sub(m.spent_generated_tokens);
        let calls = requests.saturating_sub(m.spent_requests);
        if remaining < 5.0 || tokens < 1 || calls < 1 {
            m.status = "allowance-exhausted".into();
            break;
        }
        let start = Instant::now();
        let directory = root.join(format!("candidate-{}", index + 1));
        if directory.exists() {
            std::fs::rename(
                &directory,
                root.join(format!(
                    "interrupted-{}-{}",
                    index + 1,
                    uuid::Uuid::new_v4()
                )),
            )?;
        }
        let workspace = directory.join("source");
        let state = directory.join("state");
        crate::config::private_dir(&workspace)?;
        crate::config::private_dir(&state)?;
        for (name, bytes) in &files {
            let p = workspace.join(name);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&p, bytes)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&p, std::fs::Permissions::from_mode(modes[name]))?;
            }
        }
        let candidates_left = (effort.count() - index) as u32;
        let per_seconds = (remaining / candidates_left as f64).floor().max(1.0) as u64;
        let per_tokens = (tokens / candidates_left as u64)
            .max(1)
            .min(u32::MAX as u64) as u32;
        let per_requests = (calls / candidates_left).max(1);
        let mut selected = profile.clone();
        selected.max_turns = per_requests.min(profile.max_turns);
        let mut allocation = selected.effective_inference();
        allocation.total_generated_tokens = Some(per_tokens);
        allocation.max_requests = Some(per_requests);
        selected.inference = Some(allocation);
        Config::create(&state, selected)?;
        let mut settings = prefs.clone();
        settings.project = workspace.clone();
        settings.engine_path = Some(engine.clone());
        settings.recent_projects.clear();
        settings.save(&state)?;
        if let Some(model) = &profile.local_model {
            let artifact = crate::models::artifact(data, model)?;
            crate::config::atomic_write(
                &state.join("models").join(format!("{model}.json")),
                &serde_json::to_vec(&artifact)?,
            )?;
        }
        {
            let p = Project::open(&state, &workspace)?;
            for spec in &m.checks {
                spec.contract.assertion_bytes()?;
                p.set_check(spec)?;
            }
            for r in &m.requirements {
                p.set_requirement(r)?;
            }
        }
        let stdout = std::fs::File::create(directory.join("turn.jsonl"))?;
        let stderr = std::fs::File::create(directory.join("turn.stderr"))?;
        let feedback=m.rows.last().map(|r|format!("Previous candidate observation (unverified prose is excluded): {}. Try a different focused approach from the original source.",r["verification"])).unwrap_or_default();
        let prompt = format!(
            "{goal}\n{feedback}\nCandidate {}. Use native tools. Preserve original tests. Run required independent checks. Do not claim success without current evidence.",
            index + 1
        );
        let mut command = tokio::process::Command::new(&executable);
        command
            .args(["--data-dir"])
            .arg(&state)
            .arg("--engine")
            .arg(&engine)
            .args([
                "--access",
                match prefs.access_policy {
                    Policy::Trusted => "trusted",
                    Policy::Guided => "guided",
                    Policy::ReviewOnly => "review-only",
                },
                "run",
            ])
            .arg(prompt)
            .args(["--allow-tools", "--json", "--timeout"])
            .arg(per_seconds.saturating_sub(2).max(1).to_string())
            .current_dir(&workspace)
            .stdout(stdout)
            .stderr(stderr);
        crate::process::configure(&mut command);
        m.in_flight = Some(
            json!({"index":index+1,"seconds":per_seconds,"tokens":per_tokens,"requests":per_requests,"source_directory":workspace,"state_directory":state}),
        );
        m.spent_seconds += per_seconds as f64;
        m.spent_generated_tokens += per_tokens as u64;
        m.spent_requests += per_requests;
        save(&root, &m)?;
        let mut child = command.spawn()?;
        let pid = child.id();
        let check_reserve = (per_seconds / 4).clamp(3, 60);
        let limit = Duration::from_secs(per_seconds.saturating_sub(check_reserve).max(1));
        let exit = tokio::select! {v=tokio::time::timeout(limit,child.wait())=>v.ok().and_then(|s|s.ok()).and_then(|s|s.code()),_=crate::models::cancelled(&cancel)=>None};
        #[cfg(unix)]
        if let Some(pid) = pid {
            let _ = nix::sys::signal::killpg(
                nix::unistd::Pid::from_raw(pid as i32),
                nix::sys::signal::Signal::SIGKILL,
            );
        }
        let _ = child.start_kill();
        let _ = child.wait().await;
        let verify_task = format!("candidate-{}", index + 1);
        {
            let p = Project::open(&state, &workspace)?;
            p.start_task(&verify_task, goal)?;
            for spec in &m.checks {
                p.set_check(spec)?;
            }
        }
        let mut timed_out = false;
        for requirement in &m.requirements {
            let allowance = per_seconds.saturating_sub(start.elapsed().as_secs());
            if allowance == 0 || cancel.load(Ordering::Relaxed) {
                timed_out = true;
                break;
            }
            match tokio::time::timeout(
                Duration::from_secs(allowance),
                crate::sandbox::run(
                    state.clone(),
                    workspace.clone(),
                    verify_task.clone(),
                    requirement.check_name.clone(),
                    prefs.access_policy,
                    cancel.clone(),
                ),
            )
            .await
            {
                Ok(result) => {
                    result?;
                }
                Err(_) => {
                    timed_out = true;
                    break;
                }
            }
        }
        let (verification, current, changed_tests, source_files, unsupported_changes) = {
            let p = Project::open(&state, &workspace)?;
            let (hash, now) = p.snapshot()?;
            let changed_tests = m.base_files.iter().any(|(name, hash)| {
                (name.to_lowercase().contains("test") || name.to_lowercase().contains("spec"))
                    && now.get(name).map(|b| crate::project::digest(b)).as_ref() != Some(hash)
            });
            let unsupported_changes = now
                .iter()
                .filter_map(|(name, bytes)| {
                    let content_changed = files.get(name) != Some(bytes);
                    let mode_changed =
                        p.mode(name).ok() != Some(modes.get(name).copied().unwrap_or(0o644));
                    (mode_changed || (content_changed && std::str::from_utf8(bytes).is_err()))
                        .then(|| name.clone())
                })
                .chain(
                    files
                        .iter()
                        .filter(|(name, b)| {
                            !now.contains_key(*name) && std::str::from_utf8(b).is_err()
                        })
                        .map(|(name, _)| name.clone()),
                )
                .collect::<Vec<_>>();
            (
                p.verification()?,
                hash,
                changed_tests,
                now,
                unsupported_changes,
            )
        };
        let mut charged = 0u64;
        let mut issued = 0u32;
        let mut violation = false;
        for dir in std::fs::read_dir(state.join("inference"))
            .into_iter()
            .flatten()
        {
            for f in std::fs::read_dir(dir?.path())? {
                let f = f?.path();
                if f.to_string_lossy().ends_with("-receipt.json") {
                    let receipt: Value = serde_json::from_slice(&std::fs::read(f)?)?;
                    charged = charged.saturating_add(
                        receipt["charged_generated_tokens"]
                            .as_u64()
                            .unwrap_or(per_tokens as u64),
                    );
                    issued += 1;
                    violation |= receipt["provider_budget_violation"] == true;
                }
            }
        }
        timed_out |= start.elapsed().as_secs_f64() > per_seconds as f64;
        let eligible = verification.complete
            && verification.behavioral_acceptance
            && !changed_tests
            && !timed_out
            && !cancel.load(Ordering::Relaxed)
            && !violation
            && unsupported_changes.is_empty()
            && charged <= per_tokens as u64
            && issued <= per_requests
            && current != m.baseline;
        let paths: std::collections::BTreeSet<_> =
            files.keys().chain(source_files.keys()).collect();
        let diffs:Vec<_>=paths.into_iter().filter(|name|files.get(*name)!=source_files.get(*name)).map(|name|json!({"path":name,"before_sha256":m.base_files.get(name),"after_sha256":source_files.get(name).map(|bytes|crate::project::digest(bytes)),"diff":similar::TextDiff::from_lines(&String::from_utf8_lossy(files.get(name).map(Vec::as_slice).unwrap_or(&[])),&String::from_utf8_lossy(source_files.get(name).map(Vec::as_slice).unwrap_or(&[]))).unified_diff().context_radius(3).header(&format!("a/{name}"),&format!("b/{name}")).to_string()})).collect();
        m.spent_seconds = m.spent_seconds - per_seconds as f64 + start.elapsed().as_secs_f64();
        m.spent_generated_tokens = m
            .spent_generated_tokens
            .saturating_sub(per_tokens as u64)
            .saturating_add(charged);
        m.spent_requests = m
            .spent_requests
            .saturating_sub(per_requests)
            .saturating_add(issued);
        m.in_flight = None;
        m.rows.push(json!({"index":index+1,"ancestry":m.baseline,"source_revision":current,"eligible":eligible,"changed_tests":changed_tests,"unsupported_mode_or_binary_changes":unsupported_changes,"deadline_exhausted":timed_out,"cli_exit":exit,"verification":verification,"diffs":diffs,"charged_generated_tokens":charged,"requests":issued,"wall_seconds":start.elapsed().as_secs_f64(),"source_directory":workspace,"state_directory":state,"scope":"Separate source copy; Full access retains normal host permissions and external side effects. Automatic promotion supports UTF-8 content changes with preserved permissions; other changes remain reviewable in the copy."}));
        if eligible {
            m.selected = Some(index + 1);
            m.status = "verified-candidate".into();
            save(&root, &m)?;
            break;
        }
        if cancel.load(Ordering::Relaxed) {
            m.status = "cancelled".into();
            save(&root, &m)?;
            break;
        }
        if issued == 0 && current == m.baseline {
            m.status =
                "startup-or-input-failed; inspect captured stderr and checks before retrying"
                    .into();
            save(&root, &m)?;
            break;
        }
        if m.rows.len() >= 2
            && m.rows
                .iter()
                .rev()
                .take(2)
                .all(|r| r["source_revision"] == m.baseline)
        {
            m.status = "no-source-progress; two independent attempts left source unchanged".into();
            save(&root, &m)?;
            break;
        }
        save(&root, &m)?;
    }
    if m.status == "running" {
        m.status = "no-verified-candidate".into();
    }
    save(&root, &m)?;
    Ok(m)
}
pub fn promote(data: &Path, cwd: &Path, id: &str, index: usize, policy: Policy) -> Result<Value> {
    let mut m = load(data, id)?;
    let row = m
        .rows
        .iter()
        .find(|r| r["index"].as_u64() == Some(index as u64))
        .context("Candidate does not exist")?;
    ensure!(
        row["eligible"] == true,
        "Candidate has no independent verified acceptance"
    );
    let state = PathBuf::from(
        row["state_directory"]
            .as_str()
            .context("Candidate state missing")?,
    );
    let source = PathBuf::from(
        row["source_directory"]
            .as_str()
            .context("Candidate source missing")?,
    );
    let now = {
        let p = Project::open(&state, &source)?;
        let (hash, files) = p.snapshot()?;
        ensure!(
            Some(hash.as_str()) == row["source_revision"].as_str()
                && p.verification()?.behavioral_acceptance
                && p.verification()?.complete,
            "Candidate changed since selection; reverify before applying"
        );
        files
    };
    let p = Project::open(data, cwd)?;
    ensure!(
        p.root == m.source_root && p.snapshot()?.0 == m.baseline,
        "Your project changed after the candidate fork; no files applied"
    );
    let task = format!("promote-{id}-{index}");
    p.start_task(&task, &m.goal)?;
    p.note(
        &task,
        "plan",
        "Explicitly apply reviewed verified candidate diffs",
        "user candidate promotion",
    )?;
    let before = p.snapshot()?.1;
    let paths: std::collections::BTreeSet<_> = before.keys().chain(now.keys()).cloned().collect();
    let mut proposed = Vec::new();
    for path in paths {
        if before.get(&path) == now.get(&path) {
            continue;
        }
        let old = before
            .get(&path)
            .map(|b| String::from_utf8(b.clone()))
            .transpose()?;
        let new = now
            .get(&path)
            .map(|b| String::from_utf8(b.clone()))
            .transpose()?;
        if old.is_some() {
            p.read(&task, &path, 1, 1)?;
        }
        let operation = if old.is_none() {
            "create"
        } else if new.is_none() {
            "delete"
        } else {
            "replace"
        };
        proposed.push(p.prepare_edit(
            &task,
            &path,
            None,
            old.as_deref().unwrap_or(""),
            new.as_deref().unwrap_or(""),
            operation,
            "Apply reviewed verified candidate",
        )?);
    }
    let mut applied = Vec::new();
    for change in &proposed {
        match p.apply(&change.id, policy) {
            Ok(_) => applied.push(change.id.clone()),
            Err(error) => {
                let recovery: Vec<_> = applied
                    .iter()
                    .rev()
                    .map(|id| json!({"checkpoint":id,"restored":p.undo(id).is_ok()}))
                    .collect();
                return Err(error.context(format!("Promotion stopped; rollback results {recovery:?}. Conflicting user edits are preserved.")));
            }
        }
    }
    m.status = "promoted; original project requires fresh checks".into();
    save(&path(data, id)?, &m)?;
    Ok(
        json!({"applied_checkpoints":applied,"verification":"Candidate evidence is retained separately; rerun original required checks after promotion"}),
    )
}
