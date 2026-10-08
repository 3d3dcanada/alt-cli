//! A disposable first task with real evidence, independent assertions and undo.
use crate::{
    config,
    project::{CheckSpec, Project},
    project_services::Requirement,
    verification::{Contract, Format, Kind, ReportSpec},
};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub const GOAL: &str = "Repair greeting.py so greet(name) trims spaces and returns Hello, NAME!, using friend for an empty name. Preserve the function interface. Read the file, apply an edit, and run the configured Practice behavior check. Explain the actual result.";
pub const SEED: &str = "def greet(name):\n    return 'Hello, ' + name + '!'\n";
const ASSERTION: &str = r#"import importlib.util, json, os, sys
from pathlib import Path
spec = importlib.util.spec_from_file_location('practice_greeting', Path.cwd() / 'greeting.py')
cases = []
try:
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    for name, value, expected in [('ordinary name', 'Cedar', 'Hello, Cedar!'), ('trim spaces', '  Cedar  ', 'Hello, Cedar!'), ('empty name', '', 'Hello, friend!'), ('spaces only', '   ', 'Hello, friend!')]:
        try:
            actual = module.greet(value)
            passed = actual == expected
            print(f'{name}: expected {expected!r}, got {actual!r}')
        except Exception as error:
            passed = False
            print(f'{name}: {error}')
        cases.append({'name': name, 'status': 'passed' if passed else 'failed'})
except Exception as error:
    print(f'Cannot load greeting.py: {error}')
    cases.append({'name': 'load greeting.py', 'status': 'failed'})
Path(os.environ['ALT_CHECK_REPORT']).write_text(json.dumps({'schema': 1, 'complete': True, 'tests': cases}))
sys.exit(0 if all(c['status'] == 'passed' for c in cases) else 1)
"#;

#[derive(Debug, Clone, Serialize)]
pub struct Practice {
    pub project: PathBuf,
    pub task: String,
    pub goal: String,
}

#[derive(Debug, Serialize)]
pub struct PreparedTest {
    #[serde(flatten)]
    pub lesson: Practice,
    pub state: PathBuf,
    pub source_state: PathBuf,
    pub selected_profile: String,
    pub selection: serde_json::Value,
    pub source_settings: serde_json::Value,
}

/// Reuses verified weight paths and binaries, but creates all mutable test state
/// separately. No source project, session database, settings or presets are copied
/// back. The model selection and inference/runtime settings remain explicit.
pub fn prepare_test(
    source: &Path,
    destination: &Path,
    selected: Option<&str>,
    engine: &Path,
) -> Result<PreparedTest> {
    let config = crate::config::Config::read(source)?;
    let (name, profile) = config.profile(selected)?;
    ensure!(
        profile.uncensored,
        "PC live repair tests require your explicitly selected uncensored/abliterated model; no fallback is allowed"
    );
    let mut prefs = crate::config::Preferences::load(source)?;
    prefs.engine_path = Some(
        crate::runtime::find_engine(source, engine, &prefs)
            .context("Install the agent engine before running a PC live repair test")?
            .canonicalize()?,
    );
    if profile.local_model.is_some() {
        prefs.runtime_path = Some(
            crate::runtime::find_runtime(source, &prefs)
                .context(
                    "Install or select the model runtime before running the PC live repair test",
                )?
                .canonicalize()?,
        );
    }
    let artifact = profile
        .local_model
        .as_deref()
        .map(|id| crate::models::artifact(source, id))
        .transpose()?;
    if let Some(id) = prefs.instruction_version.as_deref() {
        crate::instructions::text(source, id)?;
        crate::instructions::metadata(source, id)?;
    }
    ensure!(
        !destination.exists(),
        "Use a new isolated test state folder; existing folders are never overwritten"
    );
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir(destination)?;
    config::private_dir(destination)?;
    let destination = destination.canonicalize()?;
    if let Some(artifact) = &artifact {
        config::private_dir(&destination.join("models"))?;
        config::atomic_write(
            &destination
                .join("models")
                .join(format!("{}.json", artifact.id)),
            &serde_json::to_vec_pretty(artifact)?,
        )?;
    }
    if let Some(id) = prefs.instruction_version.as_deref() {
        let directory = destination.join("instructions").join(id);
        config::private_dir(&directory)?;
        for file in ["procedure.md", "metadata.json", "registration.json"] {
            config::atomic_write(
                &directory.join(file),
                &std::fs::read(source.join("instructions").join(id).join(file))?,
            )?;
        }
    }
    let copied = crate::config::Config {
        default_profile: name.into(),
        profiles: std::collections::BTreeMap::from([(name.into(), profile.clone())]),
    };
    copied.save(&destination)?;
    let lesson = create(&destination)?;
    prefs.project = lesson.project.clone();
    prefs.recent_projects = vec![lesson.project.clone()];
    prefs.save(&destination)?;
    let digest_file = |file: &str| {
        std::fs::read(source.join(file))
            .ok()
            .map(|bytes| crate::project::digest(&bytes))
    };
    Ok(PreparedTest {
        lesson,
        state: destination,
        source_state: source.canonicalize()?,
        selected_profile: name.into(),
        selection: serde_json::json!({"profile":profile,"artifact":artifact,"runtime":prefs.runtime,"runtime_path":prefs.runtime_path,"engine_path":prefs.engine_path,"tool_profile":prefs.tool_profile,"workflow":prefs.workflow,"active_skill":prefs.active_skill,"instruction_version":prefs.instruction_version,"scope":"Copied selection; weights are reused in place and still integrity-checked by the runtime. External weight identity remains unqualified."}),
        source_settings: serde_json::json!({"config_sha256":digest_file("config.toml"),"preferences_sha256":digest_file("preferences.toml"),"runtime_sha256":digest_file("runtime.toml")}),
    })
}

/// Each invocation creates a new folder. Existing lessons and user projects are never reset.
pub fn create(data: &Path) -> Result<Practice> {
    let id = uuid::Uuid::new_v4().to_string();
    let base = data.join("practice").join(&id);
    let project = base.join("project");
    config::private_dir(&project)?;
    config::atomic_write(&project.join("greeting.py"), SEED.as_bytes())?;
    config::atomic_write(&project.join("README.md"), format!("# Your first repair\n\n{GOAL}\n\n1. Run Practice behavior to see the real failure.\n2. Read greeting.py in Project files or ask your selected model to repair it.\n3. Run Practice behavior again and inspect all four cases.\n4. Undo the tracked edit in Task and rerun: the original failure should return.\n\nEvery new lesson gets its own folder. You can return here with Choose a project folder.\nThe assertion lives outside this editable project. Full access still has normal host permissions.\n").as_bytes())?;
    let assertion = base.join("assertion.py");
    config::atomic_write(&assertion, ASSERTION.as_bytes())?;
    let practice = Practice {
        project: project.canonicalize()?,
        task: format!("practice-{id}"),
        goal: GOAL.into(),
    };
    let p = Project::open(data, &practice.project)?;
    p.start_task(&practice.task, GOAL)?;
    p.note(&practice.task, "plan", "Read greeting.py, reproduce the failed check, repair the greeting, rerun all four cases, then try undo.", "practice setup")?;
    p.set_check(&CheckSpec {
        name: "Practice behavior".into(),
        argv: vec!["python3".into(), "{assertion}".into()],
        timeout_secs: 30,
        contract: Contract {
            kind: Kind::Tests,
            report: Some(ReportSpec {
                format: Format::Json,
                path: ".alt-practice-report.json".into(),
            }),
            assertion: Some(Contract::pin(&assertion)?),
            inputs: Default::default(),
        },
    })?;
    p.set_requirement(&Requirement {
        name: "Greeting works for all four cases".into(),
        description: GOAL.into(),
        check_name: "Practice behavior".into(),
    })?;
    p.pin("Preserve greet(name). Trim surrounding spaces; use friend for empty input. A model answer alone does not verify this repair.")?;
    config::atomic_write(
        &base.join("lesson.json"),
        &serde_json::to_vec_pretty(&practice)?,
    )?;
    Ok(practice)
}
