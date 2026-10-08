//! Checks run against a source snapshot. Isolation is explicit and never silently downgraded.
use crate::project::{CheckResult, CheckSpec, Policy, Project};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
};

#[derive(Debug, Clone, Serialize)]
pub struct Isolation {
    pub available: bool,
    pub detail: String,
}
pub async fn probe() -> Isolation {
    #[cfg(target_os = "linux")]
    {
        let mut command = base();
        command.args(["--", "/bin/true"]);
        match tokio::time::timeout(Duration::from_secs(3), command.output()).await {
            Ok(Ok(r)) if r.status.success() => Isolation {
                available: true,
                detail:
                    "Linux namespaces: private network/PID/IPC, restricted mounts, temporary home"
                        .into(),
            },
            Ok(Ok(r)) => Isolation {
                available: false,
                detail: crate::display_text(&format!(
                    "Isolation is unavailable: {}",
                    String::from_utf8_lossy(&r.stderr)
                )),
            },
            Ok(Err(e)) => Isolation {
                available: false,
                detail: format!("Install bubblewrap to enable isolated checks: {e}"),
            },
            Err(_) => Isolation {
                available: false,
                detail: "The isolation probe timed out".into(),
            },
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        Isolation {
            available: false,
            detail: "Isolated checks currently require Linux and bubblewrap".into(),
        }
    }
}
fn base() -> Command {
    let mut c = Command::new("bwrap");
    c.args([
        "--die-with-parent",
        "--new-session",
        "--unshare-all",
        "--clearenv",
        "--ro-bind",
        "/usr",
        "/usr",
        "--ro-bind-try",
        "/bin",
        "/bin",
        "--ro-bind-try",
        "/lib",
        "/lib",
        "--ro-bind-try",
        "/lib64",
        "/lib64",
        "--dir",
        "/etc",
        "--ro-bind-try",
        "/etc/ld.so.cache",
        "/etc/ld.so.cache",
        "--ro-bind-try",
        "/etc/alternatives",
        "/etc/alternatives",
        "--dir",
        "/proc",
        "--dev",
        "/dev",
        "--tmpfs",
        "/tmp",
        "--dir",
        "/home",
        "--setenv",
        "HOME",
        "/tmp",
        "--setenv",
        "PATH",
        "/usr/local/bin:/usr/bin:/bin",
        "--setenv",
        "LANG",
        "C.UTF-8",
        "--setenv",
        "PYTHONDONTWRITEBYTECODE",
        "1",
    ]);
    c.kill_on_drop(true);
    c
}
struct OwnedProcess {
    child: tokio::process::Child,
    pid: Option<u32>,
}
impl Drop for OwnedProcess {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(id) = self.pid {
            let _ = nix::sys::signal::killpg(
                nix::unistd::Pid::from_raw(id as i32),
                nix::sys::signal::Signal::SIGKILL,
            );
        }
        let _ = self.child.start_kill();
    }
}
async fn capture(
    mut reader: impl AsyncRead + Unpin,
    mut raw: Option<crate::verification::LogWriter>,
) -> (
    Vec<u8>,
    bool,
    crate::project_services::TestCounts,
    Option<String>,
    Option<crate::verification::RawLog>,
) {
    let mut saved = Vec::new();
    let mut buffer = [0; 8192];
    let mut truncated = false;
    let mut capture_error = None;
    let mut tests = crate::project_services::TestCounts::default();
    loop {
        match reader.read(&mut buffer).await {
            Ok(0) => break,
            Err(error) => {
                capture_error = Some(format!("Output read failed; evidence incomplete: {error}"));
                break;
            }
            Ok(n) => {
                tests.push(&buffer[..n]);
                if let Some(log) = &mut raw
                    && let Err(error) = log.append(&buffer[..n]).await
                {
                    capture_error = Some(format!(
                        "Raw log storage failed; evidence incomplete: {error:#}"
                    ));
                    raw = None;
                }
                let take = n.min(128 * 1024 - saved.len());
                saved.extend_from_slice(&buffer[..take]);
                truncated |= take < n;
            }
        }
    }
    let log = if let Some(raw) = raw {
        match raw.finish().await {
            Ok(log) => Some(log),
            Err(error) => {
                capture_error = Some(format!("Raw log could not be committed: {error:#}"));
                None
            }
        }
    } else {
        None
    };
    (saved, truncated, tests.finish(), capture_error, log)
}
/// The command is an explicitly registered user choice; the model cannot supply argv.
pub async fn run(
    data: PathBuf,
    cwd: PathBuf,
    task: String,
    name: String,
    policy: Policy,
    cancel: Arc<AtomicBool>,
) -> Result<CheckResult> {
    ensure!(
        policy != Policy::ReviewOnly,
        "Review only does not execute project code. Choose Guided changes to run isolated checks"
    );
    let (spec, hash, files, state) = {
        let p = Project::open(&data, &cwd)?;
        let spec = p
            .checks()?
            .into_iter()
            .find(|s| s.name == name)
            .context("Configure this check from the task menu before running it")?;
        let (hash, files) = p.snapshot()?;
        let files = files
            .into_iter()
            .map(|(path, bytes)| {
                let mode = p.mode(&path)?;
                Ok((path, bytes, mode))
            })
            .collect::<Result<Vec<_>>>()?;
        (spec, hash, files, p.state.clone())
    };
    let started = Instant::now();
    let mut result = CheckResult {
        id: uuid::Uuid::new_v4().to_string(),
        task: task.clone(),
        name,
        argv: spec.argv.clone(),
        exit_code: None,
        timed_out: false,
        cancelled: false,
        output: String::new(),
        output_truncated: false,
        snapshot: hash,
        isolation: if policy == Policy::Trusted {
            "Unisolated: trusted command in disposable source copy"
        } else {
            "Bubblewrap: network off, restricted filesystem, disposable source copy"
        }
        .into(),
        elapsed_ms: 0,
        error: None,
        environment: crate::project_services::check_environment(&cwd, &spec.argv, &spec.contract),
        tests_run: None,
        contract: spec.contract.clone(),
        structured: None,
        execution: crate::verification::ExecutionEvidence {
            project_root: Some(cwd.clone()),
            source_scope: "Observed input bytes and metadata before/after execution. Inputs are writable; stable observations do not establish immutable execution.".into(),
            ..Default::default()
        },
    };
    let spool = crate::verification::ExecutionSpool::new(&state, &result.id, &task)?;
    result.execution.receipt = Some(format!("execution/{}/receipt.json", result.id));
    spool.started_check(&result)?;
    if result
        .environment
        .keys()
        .any(|k| k.starts_with("unavailable_"))
    {
        result.error =
            Some("Installed dependency inventory is incomplete; no verified check can run".into());
    }
    let assertion_bytes = match spec.contract.assertion_bytes() {
        Ok(v) => v,
        Err(e) => {
            result.error = Some(format!("{e:#}"));
            None
        }
    };
    if policy == Policy::Guided {
        let isolation = probe().await;
        if !isolation.available {
            result.error = Some(format!(
                "{}. No command ran. Install/configure bubblewrap, or explicitly choose Trusted commands if you accept host access.",
                isolation.detail
            ));
        }
    }
    if policy == Policy::Trusted
        && !cancel.load(Ordering::Relaxed)
        && let Some(program) = spec.argv.first()
        && let Some(name) = Path::new(program).file_name().and_then(|n| n.to_str())
        && [
            "cargo", "rustc", "python3", "python", "node", "npm", "git", "bash", "sh",
        ]
        .contains(&name)
    {
        let mut version = Command::new(program);
        version
            .arg("--version")
            .current_dir(&cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        crate::process::configure(&mut version);
        if let Ok(Ok(output)) = tokio::time::timeout(Duration::from_secs(2), version.output()).await
            && output.status.success()
        {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            result.environment.insert(
                format!("observed_version_{name}"),
                crate::project::bounded(text.trim(), 2000),
            );
        }
    }
    result.cancelled = cancel.load(Ordering::Relaxed);
    if result.error.is_none() && !result.cancelled {
        let scratch = tempfile::tempdir().context("Cannot create check workspace")?;
        result.execution.snapshot_root = Some(scratch.path().into());
        if policy == Policy::Guided {
            result.execution.namespace_root = Some(PathBuf::from("/work"));
        }
        let originals = files
            .iter()
            .map(|(path, bytes, _)| (path.clone(), bytes.clone()))
            .collect::<std::collections::BTreeMap<_, _>>();
        for (path, bytes, mode) in files {
            let dest = scratch.path().join(path);
            std::fs::create_dir_all(dest.parent().context("Snapshot path")?)?;
            std::fs::write(&dest, bytes)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(mode))?;
            }
            #[cfg(not(unix))]
            let _ = mode;
        }
        let independent = tempfile::tempdir().context("Cannot stage independent assertion")?;
        if let Some(bytes) = &assertion_bytes {
            let path = independent.path().join("assertion");
            std::fs::write(&path, bytes)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o555))?;
            }
        }
        // A source copy must not supply a stale/fabricated report from a prior run.
        if let Some(report) = &spec.contract.report {
            let path = scratch.path().join(&report.path);
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            std::fs::create_dir_all(path.parent().context("Report directory")?)?;
        }
        let input_paths = originals
            .keys()
            .filter(|path| {
                spec.contract
                    .report
                    .as_ref()
                    .is_none_or(|r| r.path != **path)
            })
            .cloned()
            .collect::<Vec<_>>();
        let before = crate::verification::observe_inputs(scratch.path(), input_paths.clone())?;
        result.execution.before_manifest_sha256 = Some(spool.observations("before", &before)?);
        spool.started_check(&result)?;
        let (stdout_log, stderr_log) = spool.logs()?;
        let report_path = spec.contract.report.as_ref().map(|r| {
            if policy == Policy::Guided {
                Path::new("/work").join(&r.path)
            } else {
                scratch.path().join(&r.path)
            }
        });
        let assertion_path = if policy == Policy::Guided {
            PathBuf::from("/acceptance/assertion")
        } else {
            independent.path().join("assertion")
        };
        let argv = spec
            .argv
            .iter()
            .map(|a| match a.as_str() {
                "{assertion}" => assertion_path.display().to_string(),
                "{report}" => report_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
                _ => a.clone(),
            })
            .collect::<Vec<_>>();
        // Reuse project-local installed dependencies. In guided mode these
        // mounts are read-only; Full access keeps ordinary host permissions.
        let dependencies = ["node_modules", ".venv", "venv"]
            .into_iter()
            .filter_map(|name| {
                let path = cwd.join(name);
                std::fs::symlink_metadata(&path)
                    .ok()
                    .filter(|m| m.is_dir() && !m.is_symlink())
                    .map(|_| (name, path))
            })
            .collect::<Vec<_>>();
        let mut command = if policy == Policy::Guided {
            let mut c = base();
            c.arg("--bind").arg(scratch.path()).arg("/work");
            if assertion_bytes.is_some() {
                c.arg("--ro-bind")
                    .arg(independent.path())
                    .arg("/acceptance");
            }
            if let Some(path) = &report_path {
                c.args(["--setenv", "ALT_CHECK_REPORT"]).arg(path);
            }
            for (name, path) in &dependencies {
                c.arg("--ro-bind").arg(path).arg(format!("/work/{name}"));
            }
            c.args(["--chdir", "/work", "--"]).args(&argv);
            c
        } else {
            #[cfg(unix)]
            for (name, path) in &dependencies {
                std::os::unix::fs::symlink(path, scratch.path().join(name))?;
            }
            let mut c = Command::new(&argv[0]);
            c.args(&argv[1..])
                .current_dir(scratch.path())
                .env("LANG", "C.UTF-8");
            c.env("PYTHONDONTWRITEBYTECODE", "1");
            if let Some(path) = &report_path {
                c.env("ALT_CHECK_REPORT", path);
            }
            c
        };
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        crate::process::configure(&mut command);
        #[cfg(unix)]
        unsafe {
            command.pre_exec(|| {
                // Bound generated files and open descriptors. Wall-time and output are bounded separately.
                for (resource, limit) in [
                    (nix::libc::RLIMIT_FSIZE, 64 * 1024 * 1024),
                    (nix::libc::RLIMIT_NOFILE, 256),
                ] {
                    let value = nix::libc::rlimit {
                        rlim_cur: limit,
                        rlim_max: limit,
                    };
                    if nix::libc::setrlimit(resource, &value) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                Ok(())
            });
        }
        match command.spawn() {
            Err(e) => {
                result.error = Some(format!(
                    "Cannot start {}: {e}. Install its tools/dependencies in the selected execution environment",
                    spec.argv[0]
                ))
            }
            Ok(child) => {
                let pid = child.id();
                let mut child = OwnedProcess { child, pid };
                let mut stdout = tokio::spawn(capture(
                    child.child.stdout.take().context("Check stdout")?,
                    Some(stdout_log),
                ));
                let mut stderr = tokio::spawn(capture(
                    child.child.stderr.take().context("Check stderr")?,
                    Some(stderr_log),
                ));
                let deadline = tokio::time::sleep(Duration::from_secs(spec.timeout_secs));
                tokio::pin!(deadline);
                loop {
                    tokio::select! {
                        status=child.child.wait()=>{result.exit_code=status?.code();break;},
                        _=&mut deadline=>{result.timed_out=true;break;},
                        _=tokio::time::sleep(Duration::from_millis(50))=>{if cancel.load(Ordering::Relaxed){result.cancelled=true;break;}},
                    }
                }
                // Kill the group even after the leader exits: background grandchildren must not survive.
                drop(child);
                let a = tokio::time::timeout(Duration::from_secs(2), &mut stdout).await;
                if a.is_err() {
                    stdout.abort();
                    let _ = stdout.await;
                }
                let b = tokio::time::timeout(Duration::from_secs(2), &mut stderr).await;
                if b.is_err() {
                    stderr.abort();
                    let _ = stderr.await;
                }
                if let (
                    Ok(Ok((out, ot, out_tests, out_error, out_log))),
                    Ok(Ok((err, et, err_tests, err_error, err_log))),
                ) = (a, b)
                {
                    result.execution.stdout = out_log;
                    result.execution.stderr = err_log;
                    result.tests_run = out_tests.merge(err_tests).total();
                    result.output = format!(
                        "stdout:\n{}\nstderr:\n{}",
                        String::from_utf8_lossy(&out),
                        String::from_utf8_lossy(&err)
                    );
                    result.output_truncated = ot || et;
                    result.error = out_error.or(err_error);
                } else {
                    result.output = "Output collection did not finish after process exit".into();
                    result.output_truncated = true;
                    result.error =
                        Some("Output collection did not complete; evidence is incomplete".into());
                }
            }
        }
        if let Some(report) = &spec.contract.report {
            match crate::verification::read_report(scratch.path(), report) {
                Ok((mut outcome, bytes)) => {
                    outcome.assertion_verified = assertion_bytes.is_some()
                        && spec.contract.assertion_bytes().is_ok()
                        && std::fs::read(independent.path().join("assertion"))
                            .ok()
                            .as_ref()
                            == assertion_bytes.as_ref();
                    result.tests_run = Some(outcome.executed);
                    if outcome.failed > 0 {
                        result.error = Some(format!(
                            "Structured report contains {} failing tests",
                            outcome.failed
                        ));
                    }
                    if spec.contract.assertion.is_some() && !outcome.assertion_verified {
                        result.error = Some(
                            "Independent assertion changed during execution; evidence rejected"
                                .into(),
                        );
                    }
                    result.structured = Some(outcome);
                    if let Err(error) = spool.report(&bytes) {
                        result.error = Some(format!(
                            "Structured report could not be spooled: {error:#}. Preserve {}",
                            spool.directory.display()
                        ));
                        result.structured = None;
                    }
                }
                Err(e) => {
                    let evidence_error = format!("Structured evidence incomplete: {e:#}");
                    // A missing report can be a consequence of failing to start
                    // the runner. Keep the original, actionable cause visible.
                    result.error = Some(match result.error.take() {
                        Some(cause) => format!("{cause}\n{evidence_error}"),
                        None => evidence_error,
                    });
                }
            }
        }
        let mut patches = String::new();
        let mut after = crate::verification::InputObservation::new();
        for path in input_paths {
            let observation = crate::verification::observe_inputs(scratch.path(), [path.clone()]);
            if let Ok(current) = &observation {
                after.extend(current.clone());
            }
            match observation.as_ref().ok().and_then(|o| o.get(&path)) {
                Some(current) if before.get(&path) == Some(current) => {}
                Some(current) if before.get(&path).is_some_and(|b| b.0 == current.0) => result
                    .execution
                    .transient_or_metadata_changes
                    .push(path.clone()),
                _ => {
                    result.execution.changed_inputs.push(path.clone());
                    if !crate::verification::append_source_patch(
                        scratch.path(),
                        &path,
                        Some(&originals[&path]),
                        &mut patches,
                    )
                    .unwrap_or(false)
                    {
                        result.execution.patch_preview_truncated = true;
                    }
                }
            }
        }
        match spool.observations("after", &after) {
            Ok(hash) => result.execution.after_manifest_sha256 = Some(hash),
            Err(error) => {
                result.error = Some(format!(
                    "Execution source observations could not be durably saved: {error:#}. Recovery: {}",
                    spool.directory.display()
                ))
            }
        }
        // Existing inputs may never be exempted by declaring them outputs.
        match crate::verification::new_execution_paths(
            scratch.path(),
            &originals.keys().cloned().collect(),
            spec.contract.report.as_ref().map(|r| r.path.as_str()),
            &spec.contract.inputs.generated,
        ) {
            Ok((generated, new_inputs)) => {
                result.execution.generated_paths = generated;
                for path in &new_inputs {
                    if !crate::verification::append_source_patch(
                        scratch.path(),
                        path,
                        None,
                        &mut patches,
                    )
                    .unwrap_or(false)
                    {
                        result.execution.patch_preview_truncated = true;
                    }
                }
                result.execution.changed_inputs.extend(new_inputs);
            }
            Err(error) => {
                result
                    .execution
                    .changed_inputs
                    .push("<output inventory incomplete>".into());
                result.error = Some(format!("Execution source inventory incomplete: {error:#}"));
            }
        }
        result.execution.observed_inputs_stable = result.execution.changed_inputs.is_empty()
            && result.execution.transient_or_metadata_changes.is_empty();
        if !result.execution.observed_inputs_stable {
            let reason = format!(
                "Check execution changed input source or file identity; original source is not verified. Changed inputs: {:?}; transient/metadata changes: {:?}. Review execution/{}/source.patch (bounded textual preview; incomplete: {}) and explicitly apply/recheck.",
                result.execution.changed_inputs,
                result.execution.transient_or_metadata_changes,
                result.id,
                result.execution.patch_preview_truncated
            );
            result.error = Some(match result.error.take() {
                Some(previous) => format!("{previous}\n{reason}"),
                None => reason,
            });
            if let Err(error) = spool.patch(patches.as_bytes()) {
                result.error = Some(format!(
                    "{}\nPatch persistence failed: {error:#}",
                    result.error.unwrap_or_default()
                ));
            }
        }
    }
    if result.exit_code == Some(0)
        && result.tests_run == Some(0)
        && matches!(
            spec.contract.kind,
            crate::verification::Kind::Tests | crate::verification::Kind::Custom
        )
    {
        result.error = Some("The runner reported zero tests. Configure a check that actually exercises the required behavior.".into());
    }
    if !crate::project_services::environment_current(&result, &cwd, &spec.argv)
        && result.error.is_none()
    {
        result.error=Some("Execution environment or declared input changed during the check; rerun against reviewed current inputs".into());
    }
    result.elapsed_ms = started.elapsed().as_millis();
    spool.finish_check(result.clone())?;
    // The completed receipt is already durable. Busy project attachment is
    // deferred until the next successful open; never rerun the command.
    if let Err(error) = Project::open(&data, &cwd)
        && !error.chain().any(|e| {
            e.downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::WouldBlock)
        })
    {
        return Err(error.context(format!(
            "Execution receipt saved at {}; project attachment requires recovery",
            state.join("execution").join(&result.id).display()
        )));
    }
    Ok(result)
}
pub fn suggested(cwd: &Path) -> Vec<CheckSpec> {
    let mut specs = Vec::new();
    if cwd.join("Cargo.toml").is_file() {
        specs.push(CheckSpec {
            name: "Rust tests".into(),
            argv: vec!["cargo".into(), "test".into(), "--offline".into()],
            timeout_secs: 120,
            contract: Default::default(),
        });
    }
    // A manifest is not evidence that a test script or a Python suite exists.
    if let Ok(file) = std::fs::File::open(cwd.join("package.json")) {
        use std::io::Read;
        let mut body = String::new();
        if file.take(256 * 1024).read_to_string(&mut body).is_ok()
            && let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&body)
            && manifest["scripts"]["test"]
                .as_str()
                .is_some_and(|s| !s.trim().is_empty())
        {
            let manager = manifest["packageManager"]
                .as_str()
                .unwrap_or("npm")
                .split('@')
                .next()
                .unwrap_or("npm");
            let program = if matches!(manager, "yarn" | "pnpm") {
                manager
            } else {
                "npm"
            };
            specs.push(CheckSpec {
                name: "JavaScript tests".into(),
                argv: vec![program.into(), "test".into()],
                timeout_secs: 120,
                contract: Default::default(),
            });
        }
    }
    let python_suite = cwd.join("pyproject.toml").is_file()
        || cwd.join("test.py").is_file()
        || std::fs::read_dir(cwd.join("tests")).is_ok_and(|items| {
            items
                .flatten()
                .any(|e| e.path().extension().is_some_and(|ext| ext == "py"))
        });
    if python_suite {
        let pyproject = std::fs::read_to_string(cwd.join("pyproject.toml")).unwrap_or_default();
        let pytest = cwd.join("pytest.ini").is_file() || pyproject.contains("[tool.pytest.");
        specs.push(CheckSpec {
            name: if pytest {
                "Python pytest"
            } else {
                "Python tests"
            }
            .into(),
            argv: if pytest {
                vec![
                    "python3".into(),
                    "-m".into(),
                    "pytest".into(),
                    "--junitxml".into(),
                    "{report}".into(),
                ]
            } else {
                vec![
                    "python3".into(),
                    "-m".into(),
                    "unittest".into(),
                    "discover".into(),
                ]
            },
            timeout_secs: 120,
            contract: if pytest {
                crate::verification::Contract {
                    kind: crate::verification::Kind::Tests,
                    report: Some(crate::verification::ReportSpec {
                        format: crate::verification::Format::Junit,
                        path: ".alt-pytest.xml".into(),
                    }),
                    assertion: None,
                    inputs: Default::default(),
                }
            } else {
                Default::default()
            },
        });
    }
    specs
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalResult {
    pub error: Option<String>,
    pub id: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub cancelled: bool,
    pub output: String,
    pub output_truncated: bool,
    pub tests_run: Option<u64>,
    pub elapsed_ms: u128,
    pub undoable: bool,
    #[serde(default)]
    pub execution: crate::verification::ExecutionEvidence,
}
/// Deliberately unrestricted, explicit Full access path. No false sandbox/undo promise.
pub async fn terminal(
    data: &Path,
    cwd: &Path,
    task: &str,
    text: &str,
    timeout: u64,
    cancel: Arc<AtomicBool>,
) -> Result<TerminalResult> {
    let mut command = Command::new("/bin/bash");
    command
        .args(["-c", text])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::process::configure(&mut command);
    let started = Instant::now();
    let mut result = TerminalResult {
        error: None,
        id: uuid::Uuid::new_v4().to_string(),
        exit_code: None,
        timed_out: false,
        cancelled: false,
        output: String::new(),
        output_truncated: false,
        tests_run: None,
        elapsed_ms: 0,
        undoable: false,
        execution: crate::verification::ExecutionEvidence {
            command: Some(text.to_owned()),
            ..Default::default()
        },
    };
    let state = {
        let project = Project::open(data, cwd)?;
        project.state.clone()
    };
    let spool = crate::verification::ExecutionSpool::new(&state, &result.id, task)?;
    result.execution.receipt = Some(format!("execution/{}/receipt.json", result.id));
    spool.started_terminal(&result)?;
    let (stdout_log, stderr_log) = spool.logs()?;
    let raw = command
        .spawn()
        .context("Could not start bash for Full access terminal")?;
    let pid = raw.id();
    let mut child = OwnedProcess { child: raw, pid };
    let mut stdout = tokio::spawn(capture(
        child.child.stdout.take().context("Terminal stdout")?,
        Some(stdout_log),
    ));
    let mut stderr = tokio::spawn(capture(
        child.child.stderr.take().context("Terminal stderr")?,
        Some(stderr_log),
    ));

    let deadline = tokio::time::sleep(Duration::from_secs(timeout));
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            status=child.child.wait()=>{result.exit_code=status?.code();break;},
            _=&mut deadline=>{result.timed_out=true;break;},
            _=tokio::time::sleep(Duration::from_millis(50))=>if cancel.load(Ordering::Relaxed){result.cancelled=true;break;},
        }
    }
    drop(child);
    let out = tokio::time::timeout(Duration::from_secs(2), &mut stdout).await;
    if out.is_err() {
        stdout.abort();
        let _ = stdout.await;
    }
    let err = tokio::time::timeout(Duration::from_secs(2), &mut stderr).await;
    if err.is_err() {
        stderr.abort();
        let _ = stderr.await;
    }
    match (out, err) {
        (
            Ok(Ok((out, ot, out_tests, out_error, out_log))),
            Ok(Ok((err, et, err_tests, err_error, err_log))),
        ) => {
            result.execution.stdout = out_log;
            result.execution.stderr = err_log;
            result.tests_run = out_tests.merge(err_tests).total();
            result.output = format!(
                "stdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&out),
                String::from_utf8_lossy(&err)
            );
            result.output_truncated = ot || et;
            result.error = out_error.or(err_error);
        }
        _ => {
            result.output = "Output collection stopped".into();
            result.output_truncated = true;
            result.error = Some("Output collection incomplete".into());
        }
    }
    result.elapsed_ms = started.elapsed().as_millis();
    spool.finish_terminal(result.clone())?;
    if let Err(error) = Project::open(data, cwd)
        && !error.chain().any(|e| {
            e.downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::WouldBlock)
        })
    {
        return Err(error.context(format!(
            "Terminal receipt saved at {}; attachment requires recovery",
            state.join("execution").join(&result.id).display()
        )));
    }
    Ok(result)
}
