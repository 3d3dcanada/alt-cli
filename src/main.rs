use alt_cli::{
    config::{Config, Preferences, Profile, Provider, data_dir, private_dir},
    display_text,
    engine::{Engine, Event, response_value, update_text},
    models, runtime,
    store::{Session, Store},
    tui,
};
use anyhow::{Context, Result, bail, ensure};
use clap::{Parser, Subcommand};
use serde_json::json;
use std::{io::Write, path::PathBuf, time::Duration};

#[derive(Parser)]
#[command(
    name = "alt",
    version,
    about = "A terminal workspace for your local models"
)]
struct Args {
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    #[arg(long, global = true)]
    profile: Option<String>,
    #[arg(long, global = true, default_value = "goose")]
    engine: PathBuf,
    /// Access for headless commands; use Settings in the TUI. Trusted enables full host terminal access.
    #[arg(long, global = true, value_enum)]
    access: Option<alt_cli::project::Policy>,
    #[command(subcommand)]
    command: Option<Action>,
}

#[derive(Subcommand)]
enum Action {
    /// Embedded source, compiler and target provenance; does not open saved state.
    BuildInfo,
    /// Optional language-server references or reviewable identifier rename.
    Language {
        #[arg(long)]
        server: PathBuf,
        #[arg(long)]
        server_arg: Vec<String>,
        path: String,
        #[arg(long)]
        line: u32,
        #[arg(long)]
        column: u32,
        #[arg(long)]
        rename: Option<String>,
        #[arg(long)]
        apply: bool,
    },
    /// Plain text interactive conversation, suitable for basic terminals and screen readers.
    Plain {
        #[arg(long)]
        resume: Option<String>,
    },
    /// Measure the exact selected uncensored model and runtime; saves a report.
    Benchmark,
    /// Measure selected contexts and owned-runtime cancellation/restart without changing settings.
    Qualify {
        #[arg(long, value_delimiter = ',', default_value = "2048,4096,8192")]
        contexts: Vec<u32>,
        #[arg(long, default_value_t = 1)]
        repeats: usize,
    },
    /// Choose the native schemas shown to the selected model; access mode is unchanged.
    Tools {
        #[arg(value_enum)]
        focus: alt_cli::toolbox::ToolProfile,
    },
    /// Configure managed inference without changing the selected model.
    Runtime {
        #[arg(long)]
        gpu_layers: Option<i32>,
        #[arg(long)]
        threads: Option<usize>,
        #[arg(long)]
        batch: Option<u32>,
        #[arg(long)]
        cache_k: Option<String>,
        #[arg(long)]
        cache_v: Option<String>,
        #[arg(long)]
        flash_attention: Option<bool>,
        /// Override enable_thinking only for templates that support it.
        #[arg(long)]
        thinking: Option<bool>,
        #[arg(long, conflicts_with = "thinking")]
        thinking_default: bool,
    },
    /// Versioned build, Git, HTTP, browser and security workflows.
    Packs {
        #[command(subcommand)]
        command: PackAction,
    },
    /// Manage explicitly selected external MCP tools.
    Extensions {
        #[command(subcommand)]
        command: ExtensionAction,
    },
    #[command(hide = true)]
    ExternalServer {
        #[arg(long)]
        name: String,
        #[arg(long)]
        socket: PathBuf,
    },
    /// Backups, restore, storage usage and redacted diagnostics.
    State {
        #[command(subcommand)]
        command: StateAction,
    },
    /// Persistent PTY jobs and interactive terminal attachment (Full access).
    Jobs {
        #[command(subcommand)]
        command: JobAction,
    },
    #[command(hide = true)]
    JobWorker {
        #[arg(long)]
        id: String,
    },

    /// Hardware, runtime ABI, isolation, and conservative memory guidance; no inference.
    Hardware,
    /// Evaluate native tool use with the explicitly selected uncensored model.
    Evaluate,
    /// Project progress, check configuration, memory, diffs and undo.
    Task {
        #[command(subcommand)]
        command: TaskAction,
    },
    #[command(hide = true)]
    ToolServer {
        #[arg(long, value_enum, default_value = "all")]
        tool_profile: alt_cli::toolbox::ToolProfile,
        #[arg(long)]
        socket: PathBuf,
    },
    /// Create a local profile; never overwrites an existing configuration.
    Init {
        #[arg(long)]
        model: String,
        #[arg(long, value_enum, default_value = "openai")]
        provider: Provider,
        /// OpenAI API root (including /v1) or Ollama server root.
        #[arg(long)]
        endpoint: Option<String>,
        #[arg(long, default_value_t = 16384)]
        context: u32,
        #[arg(long, default_value_t = 12)]
        max_turns: u32,
        /// Record your/publisher's uncensored claim (not independently verified).
        #[arg(long)]
        uncensored: bool,
        #[arg(long)]
        api_key_env: Option<String>,
    },
    /// Check the model endpoint and ACP engine without running inference.
    Doctor,
    /// Browse, import, or download models; no subcommand lists server models.
    Models {
        #[command(subcommand)]
        command: Option<ModelAction>,
    },
    /// Install a pinned agent engine or CPU inference runtime in Alt's data folder.
    Install {
        #[arg(value_enum)]
        component: InstallComponent,
    },
    /// List saved sessions.
    Sessions,
    /// Export a saved session's events as JSON Lines.
    Export { session: String },
    /// Open the terminal UI (also the default command).
    Tui {
        #[arg(long)]
        resume: Option<String>,
    },
    /// Run one prompt; tool requests are denied unless --allow-tools is supplied.
    Run {
        prompt: String,
        #[arg(long)]
        resume: Option<String>,
        /// Approve requested actions within the selected access profile for this invocation.
        #[arg(long)]
        allow_tools: bool,
        /// Emit structured events instead of readable text.
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = 600)]
        timeout: u64,
    },
}

#[derive(Subcommand)]
enum ExtensionAction {
    List,
    Add { file: PathBuf },
    Check { name: String },
    Select { name: String, tools: Vec<String> },
    Disable { name: String },
    Remove { name: String },
}

#[derive(Subcommand)]
enum StateAction {
    /// Preview or archive old conversations already marked archived by the user.
    History {
        #[arg(long, default_value_t = 30)]
        days: u64,
        #[arg(long)]
        apply: bool,
    },
    /// Restore a compressed retained conversation without overwriting existing history.
    RestoreHistory {
        archive: PathBuf,
    },
    Backup {
        destination: PathBuf,
    },
    Restore {
        archive: PathBuf,
        destination: PathBuf,
    },
    Usage,
    Diagnostics,
    Retain {
        #[arg(long, default_value_t = 30)]
        days: u64,
        #[arg(long)]
        apply: bool,
    },
}

#[derive(Subcommand)]
enum JobAction {
    List,
    Start {
        command: String,
        #[arg(long, default_value = "Terminal job")]
        name: String,
        #[arg(long, default_value_t = 0)]
        timeout: u64,
        #[arg(long)]
        wait: bool,
    },
    Stop {
        id: String,
    },
    Restart {
        id: String,
    },
    Logs {
        id: String,
    },
    Attach {
        id: String,
    },
    Input {
        id: String,
        text: String,
    },
    Resize {
        id: String,
        cols: u16,
        rows: u16,
    },
    Health {
        id: String,
        url: String,
    },
}

#[derive(Subcommand)]
enum TaskAction {
    /// Declare check purpose and structured evidence; pins an optional external assertion.
    Contract {
        name: String,
        #[arg(long, value_enum)]
        kind: alt_cli::verification::Kind,
        #[arg(long, value_enum)]
        format: Option<alt_cli::verification::Format>,
        #[arg(long, requires = "format")]
        report: Option<String>,
        #[arg(long)]
        assertion: Option<PathBuf>,
    },
    /// Define a required outcome and the configured check that verifies it.
    Require {
        name: String,
        #[arg(long)]
        check: String,
        #[arg(long, default_value = "")]
        description: String,
    },
    Unrequire {
        name: String,
    },
    Verify {
        #[arg(long)]
        run: bool,
    },
    Pin {
        text: String,
    },
    Unpin {
        id: String,
    },
    Context,
    Map,
    Index,

    Status,
    Changes,
    Diff {
        id: String,
    },
    Undo {
        id: String,
    },
    UndoAll {
        task: String,
    },
    Checks,
    ConfigureCheck {
        name: String,
        #[arg(long, default_value_t = 120)]
        timeout: u64,
        #[arg(required = true, last = true)]
        argv: Vec<String>,
    },
    Check {
        name: String,
    },
    Memory {
        query: Option<String>,
    },
    Remember {
        text: String,
    },
    Export,
}

#[derive(Subcommand)]
enum PackAction {
    List,
    Run {
        pack: String,
        #[arg(long, default_value = "{}")]
        input: String,
    },
    Findings {
        #[arg(long, default_value = "markdown")]
        format: String,
    },
    Retest {
        finding: String,
        evidence: String,
    },
}

#[derive(Subcommand)]
enum ModelAction {
    /// Unregister a model; optionally delete verified managed weights, never imported originals.
    Remove {
        id: String,
        #[arg(long)]
        delete_weights: bool,
    },
    /// Copy managed weights to a new cache and switch verified records; retain old copies.
    Relocate { destination: PathBuf },
    /// Show active cache and its last relocation receipt.
    Cache,
    /// List imported and downloaded model files.
    Local,
    /// Search public Hugging Face GGUF repositories.
    Search {
        query: String,
        /// Include models without uncensored/abliterated publisher labels.
        #[arg(long)]
        all: bool,
    },
    /// List complete GGUF artifacts with their pinned revisions and SHA256.
    Files { repository: String },
    /// Download and verify an exact file; interrupted downloads can resume.
    Download {
        repository: String,
        #[arg(long)]
        file: String,
        /// Select this local model for new conversations.
        #[arg(long = "use")]
        select: bool,
    },
    /// Add a GGUF file without copying, moving, or modifying the original.
    Import {
        path: PathBuf,
        /// Record the publisher's uncensored claim.
        #[arg(long)]
        uncensored: bool,
        #[arg(long = "use")]
        select: bool,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum InstallComponent {
    Engine,
    Runtime,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("alt: {}", display_text(&format!("{error:#}")));
        std::process::exit(1);
    }
}

async fn model_ids(profile: &Profile) -> Result<Vec<String>> {
    models::inventory(profile).await
}

async fn run() -> Result<()> {
    let args = Args::parse();
    if matches!(args.command, Some(Action::BuildInfo)) {
        println!("{}", alt_cli::BUILD_INFO);
        return Ok(());
    }
    if let Some(Action::ToolServer {
        socket,
        tool_profile,
    }) = &args.command
    {
        return alt_cli::toolbox::stdio(socket, *tool_profile).await;
    }
    let root = args.data_dir.unwrap_or_else(data_dir);
    private_dir(&root)?;
    let root = root.canonicalize()?;
    let command = args.command.unwrap_or(Action::Tui { resume: None });
    match command {
        Action::Language {
            server,
            server_arg,
            path,
            line,
            column,
            rename,
            apply,
        } => {
            let cwd = std::env::current_dir()?;
            let policy = args
                .access
                .unwrap_or(Preferences::load(&root)?.access_policy);
            let value = alt_cli::language::query(
                &root,
                &cwd,
                alt_cli::language::Query {
                    server: &server,
                    args: &server_arg,
                    path: &path,
                    line,
                    column,
                    new_name: rename.as_deref(),
                },
                policy,
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&value)?);
            if apply {
                let ids = value["changes"]
                    .as_array()
                    .context("--apply requires --rename")?
                    .iter()
                    .filter_map(|c| c["id"].as_str().map(str::to_owned))
                    .collect::<Vec<_>>();
                println!("{}", alt_cli::language::apply(&root, &cwd, &ids, policy)?);
            }
            return Ok(());
        }
        Action::Runtime {
            gpu_layers,
            threads,
            batch,
            cache_k,
            cache_v,
            flash_attention,
            thinking,
            thinking_default,
        } => {
            let mut p = Preferences::load(&root)?;
            if let Some(v) = gpu_layers {
                p.runtime.gpu_layers = v;
            }
            if let Some(v) = threads {
                p.runtime.threads = v;
            }
            if let Some(v) = batch {
                p.runtime.batch = v;
            }
            if let Some(v) = cache_k {
                p.runtime.cache_k = v;
            }
            if let Some(v) = cache_v {
                p.runtime.cache_v = v;
            }
            if let Some(v) = flash_attention {
                p.runtime.flash_attention = v;
            }
            if thinking_default {
                p.runtime.thinking = None;
            } else if let Some(v) = thinking {
                p.runtime.thinking = Some(v);
            }
            p.runtime.validate()?;
            p.save(&root)?;
            println!("{}", serde_json::to_string_pretty(&p.runtime)?);
            return Ok(());
        }
        Action::Tools { focus } => {
            let mut preferences = Preferences::load(&root)?;
            preferences.tool_profile = focus;
            preferences.save(&root)?;
            println!(
                "Tool focus saved: {}. Applies to new connections; Full access remains available.",
                focus.label()
            );
            return Ok(());
        }
        Action::Qualify {
            ref contexts,
            repeats,
        } => {
            let config = Config::read(&root)?;
            let (_, profile) = config.profile(args.profile.as_deref())?;
            let preferences = Preferences::load(&root)?;
            let cancel = alt_cli::qualification::fresh_cancel();
            let operation = alt_cli::qualification::run(
                &root,
                &preferences,
                profile,
                contexts,
                repeats,
                cancel.clone(),
            );
            tokio::pin!(operation);
            let report = tokio::select! {r=&mut operation=>r?,_=alt_cli::process::shutdown_signal()=>{cancel.store(true,std::sync::atomic::Ordering::Relaxed);operation.await?}};
            println!("{}", serde_json::to_string_pretty(&report)?);
            ensure!(
                report["status"] == "completed"
                    && report["rows"]
                        .as_array()
                        .is_some_and(|rows| rows.iter().all(|r| r["generation_passed"] == true
                            && (profile.local_model.is_none()
                                || r["owned_lifecycle_passed"] == true))),
                "Qualification contains failed or cancelled attempts; report retained"
            );
            return Ok(());
        }
        Action::Benchmark => {
            let config = Config::read(&root)?;
            let (_, profile) = config.profile(args.profile.as_deref())?;
            let (cancel, _) = transfer_status();
            let r =
                alt_cli::benchmark::run(&root, &Preferences::load(&root)?, profile, cancel).await?;
            println!("{}", serde_json::to_string_pretty(&r)?);
            ensure!(
                r["error"].is_null(),
                "Benchmark did not finish; report was saved"
            );
            return Ok(());
        }
        Action::Packs { command } => {
            let cwd = std::env::current_dir()?;
            match command {
                PackAction::List => println!(
                    "{}",
                    serde_json::to_string_pretty(&alt_cli::packs::catalog())?
                ),
                PackAction::Run { pack, input } => {
                    let (cancel, _) = transfer_status();
                    let r = alt_cli::packs::run(
                        &root,
                        &cwd,
                        &pack,
                        serde_json::from_str(&input)?,
                        args.access
                            .unwrap_or(Preferences::load(&root)?.access_policy),
                        cancel,
                    )
                    .await?;
                    println!("{}", serde_json::to_string_pretty(&r)?);
                    ensure!(
                        r.status == "passed",
                        "Workflow {}. Evidence saved as {}",
                        r.status,
                        r.id
                    );
                }
                PackAction::Findings { format } => {
                    println!("{}", alt_cli::packs::export(&root, &cwd, &format)?)
                }
                PackAction::Retest { finding, evidence } => {
                    alt_cli::packs::retest(&root, &cwd, &finding, &evidence)?
                }
            }
            return Ok(());
        }
        Action::ToolServer {
            socket,
            tool_profile,
        } => return alt_cli::toolbox::stdio(&socket, tool_profile).await,
        Action::ExternalServer { name, socket } => {
            return alt_cli::extensions::proxy(&root, &name, &socket).await;
        }
        Action::Extensions { command } => {
            let value = match command {
                ExtensionAction::List => json!(alt_cli::extensions::list(&root)?),
                ExtensionAction::Add { file } => {
                    let c: alt_cli::extensions::Connection =
                        serde_json::from_slice(&std::fs::read(file)?)?;
                    ensure!(
                        !c.enabled,
                        "Add disabled first, check its tools, then select the tools to enable"
                    );
                    alt_cli::extensions::save(&root, &c)?;
                    json!({"saved":true,"enabled":false})
                }
                ExtensionAction::Check { name } => alt_cli::extensions::probe(&root, &name).await?,
                ExtensionAction::Select { name, tools } => {
                    alt_cli::extensions::select(&root, &name, tools)?;
                    json!({"saved":true,"scope":"Selected tools are available in Full access after reconnect"})
                }
                ExtensionAction::Disable { name } => {
                    let mut c = alt_cli::extensions::load(&root, &name)?;
                    c.enabled = false;
                    alt_cli::extensions::save(&root, &c)?;
                    json!({"disabled":true})
                }
                ExtensionAction::Remove { name } => {
                    alt_cli::extensions::remove(&root, &name)?;
                    json!({"removed":true})
                }
            };
            println!("{}", serde_json::to_string_pretty(&value)?);
            return Ok(());
        }
        Action::JobWorker { id } => return alt_cli::jobs::worker(&root, &id).await,
        Action::Jobs { command } => return job_action(&root, command, args.access).await,
        Action::State { command } => {
            let value = match command {
                StateAction::Backup { destination } => {
                    alt_cli::storage::backup(&root, &destination)?
                }
                StateAction::Restore {
                    archive,
                    destination,
                } => alt_cli::storage::restore(&archive, &destination)?,
                StateAction::History { days, apply } => {
                    Store::open(&root)?.retain_archived(&root, days, apply)?
                }
                StateAction::RestoreHistory { archive } => {
                    json!({"restored_session":Store::open(&root)?.restore_history(&archive)?})
                }
                StateAction::Usage => alt_cli::storage::usage(&root)?,
                StateAction::Diagnostics => alt_cli::storage::diagnostics(&root)?,
                StateAction::Retain { days, apply } => {
                    alt_cli::storage::retain(&root, days, apply)?
                }
            };
            println!("{}", serde_json::to_string_pretty(&value)?);
            return Ok(());
        }
        Action::Hardware => {
            println!(
                "{}",
                serde_json::to_string_pretty(&alt_cli::hardware::inspect().await)?
            );
            return Ok(());
        }
        Action::Task { command } => return task_action(&root, command, args.access).await,
        Action::Evaluate => {
            let config = Config::read(&root)?;
            let (_, profile) = config.profile(args.profile.as_deref())?;
            let (cancel, _) = transfer_status();
            let report =
                alt_cli::hardware::evaluate(&root, &Preferences::load(&root)?, profile, cancel)
                    .await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            ensure!(
                report.error.is_none(),
                "Capability probe failed; its report was saved"
            );
            return Ok(());
        }
        Action::Init {
            model,
            provider,
            endpoint,
            context,
            max_turns,
            uncensored,
            api_key_env,
        } => {
            let endpoint = endpoint.unwrap_or_else(|| match provider {
                Provider::Openai => "http://127.0.0.1:8080/v1".into(),
                Provider::Ollama => "http://127.0.0.1:11434".into(),
            });
            Config::create(
                &root,
                Profile {
                    provider,
                    endpoint,
                    model,
                    context_tokens: context,
                    max_turns,
                    uncensored,
                    api_key_env,
                    local_model: None,
                },
            )?;
            println!("Created {}", root.join("config.toml").display());
            return Ok(());
        }
        Action::Sessions => {
            for (id, profile, cwd) in Store::open(&root)?.list()? {
                println!("{}", display_text(&format!("{id}  {profile}  {cwd}")));
            }
            return Ok(());
        }
        Action::Export { session } => {
            Store::open(&root)?.visit_history(&session, |event| {
                println!("{event}");
                Ok(())
            })?;
            return Ok(());
        }
        Action::Tui { resume } => {
            ensure!(
                args.access.is_none(),
                "Choose access in TUI Settings, or use --access with a headless command such as run or task check"
            );
            return tui::run(root, args.engine, args.profile, resume).await;
        }
        Action::Models {
            command: Some(command),
        } => return model_action(&root, command).await,
        Action::Install { component } => {
            let component = match component {
                InstallComponent::Engine => runtime::Component::Engine,
                InstallComponent::Runtime => runtime::Component::Inference,
            };
            let (cancel, mut progress) = transfer_status();
            let path = runtime::install(&root, component, cancel, &mut progress).await?;
            let mut preferences = Preferences::load(&root)?;
            match component {
                runtime::Component::Engine => preferences.engine_path = Some(path.clone()),
                runtime::Component::Inference => preferences.runtime_path = Some(path.clone()),
            }
            preferences.save(&root)?;
            println!(
                "{}: {}",
                component.description(),
                display_text(&path.display().to_string())
            );
            return Ok(());
        }
        _ => {}
    }
    let config = Config::read(&root)?;
    let (name, profile) = config.profile(args.profile.as_deref())?;
    if matches!(command, Action::Models { .. }) {
        if profile.local_model.is_some() {
            for model in models::library(&root)? {
                println!("{}  {}", model.id, display_text(&model.name));
            }
            return Ok(());
        }
        for id in model_ids(profile).await? {
            println!("{}", display_text(&id));
        }
        return Ok(());
    }
    private_dir(&root)?;
    let root = root.canonicalize()?;
    let preferences = Preferences::load(&root)?;
    let binary = runtime::find_engine(&root, &args.engine, &preferences).context(
        "Agent engine missing. Open Alt and install it from Home, or pass --engine PATH",
    )?;
    let cwd = std::env::current_dir()?.canonicalize()?;
    if matches!(command, Action::Doctor) {
        let mut profile = profile.clone();
        let _local = if profile.local_model.is_some() {
            let local = runtime::LocalRuntime::start(
                &root,
                &preferences,
                &profile,
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                |_| {},
            )
            .await?;
            profile.endpoint = local.endpoint.clone();
            Some(local)
        } else {
            None
        };
        let models = model_ids(&profile).await?;
        ensure!(
            models.contains(&profile.model),
            "Configured model is absent from the server's inventory; use `alt models`"
        );
        let mut engine = Engine::goose_with_policy(
            &binary,
            &root,
            &cwd,
            &profile,
            args.access.unwrap_or(preferences.access_policy),
        )
        .await?;
        let result = engine.initialize().await;
        engine.shutdown().await;
        let info = result?;
        println!(
            "{}",
            display_text(&format!(
                "Endpoint reachable; model listed: {}\nACP v1 ready: {} {}\nContext: {} tokens; turn limit: {}\nInference/tool quality has not been tested by this check.",
                profile.model,
                info["agentInfo"]["name"].as_str().unwrap_or("unknown"),
                info["agentInfo"]["version"].as_str().unwrap_or("unknown"),
                profile.context_tokens,
                profile.max_turns
            ))
        );
        return Ok(());
    }
    let store = Store::open(&root)?;
    let resume = match &command {
        Action::Tui { resume } | Action::Plain { resume } | Action::Run { resume, .. } => {
            resume.as_deref()
        }
        _ => None,
    };
    let saved = resume.map(|id| store.get(id)).transpose()?;
    // A resumed session retains its original model/endpoint/context configuration.
    let (chosen_name, chosen_profile, chosen_cwd) = if let Some(saved) = &saved {
        if let Some(requested) = args.profile.as_deref() {
            ensure!(
                requested == saved.profile_name,
                "Resume uses its saved profile"
            );
        }
        (
            saved.profile_name.as_str(),
            &saved.profile,
            PathBuf::from(&saved.cwd),
        )
    } else {
        (name, profile, cwd)
    };
    ensure!(chosen_cwd.is_dir(), "Session workspace no longer exists");
    let mut effective = chosen_profile.clone();
    let mut local_runtime = if effective.local_model.is_some() {
        let local = runtime::LocalRuntime::start(
            &root,
            &preferences,
            &effective,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            |_| {},
        )
        .await?;
        effective.endpoint = local.endpoint.clone();
        Some(local)
    } else {
        None
    };
    let mut engine = Engine::goose_with_policy(
        &binary,
        &root,
        &chosen_cwd,
        &effective,
        args.access.unwrap_or(preferences.access_policy),
    )
    .await?;
    let info = engine.initialize().await?;
    let session = if let Some(saved) = saved {
        ensure!(
            info["agentCapabilities"]["loadSession"] == true,
            "Engine does not support session recovery"
        );
        engine
            .restore_session(std::path::Path::new(&saved.cwd))
            .await?;
        saved
    } else {
        let response = engine.new_session(&chosen_cwd).await?;
        let session = Session {
            id: uuid::Uuid::new_v4().to_string(),
            engine_id: response["sessionId"]
                .as_str()
                .context("Engine returned no session ID")?
                .into(),
            cwd: chosen_cwd.to_string_lossy().into_owned(),
            profile_name: chosen_name.into(),
            profile: chosen_profile.clone(),
        };
        store.create(&session)?;
        session
    };
    engine.set_task(&session.id);
    let result = match command {
        Action::Plain { .. } => {
            println!(
                "Alt · {} · {}\nSession: {}\nType a request; /quit exits. Tools ask before each action. Full history is saved.",
                session.profile.model,
                args.access.unwrap_or(preferences.access_policy).label(),
                session.id
            );
            loop {
                print!("\nYou> ");
                std::io::stdout().flush()?;
                let line = tokio::task::spawn_blocking(|| {
                    let mut s = String::new();
                    std::io::stdin().read_line(&mut s).map(|n| (n, s))
                })
                .await??;
                if line.0 == 0 || line.1.trim() == "/quit" {
                    break;
                }
                if line.1.trim().is_empty() {
                    continue;
                }
                headless(
                    &mut engine,
                    &store,
                    &session,
                    line.1.trim(),
                    Approval::Ask,
                    false,
                    600,
                )
                .await?;
            }
            Ok(())
        }
        Action::Run {
            prompt,
            allow_tools,
            json,
            timeout,
            ..
        } => {
            headless(
                &mut engine,
                &store,
                &session,
                &prompt,
                if allow_tools {
                    Approval::Allow
                } else {
                    Approval::Deny
                },
                json,
                timeout,
            )
            .await
        }
        _ => unreachable!(),
    };
    if let Err(error) = &result {
        let _ = store.append(
            &session.id,
            &json!({"type":"error","text":error.to_string()}),
        );
    }
    engine.shutdown().await;
    if let Some(local) = &mut local_runtime {
        local.stop().await;
    }
    result
}

fn transfer_status() -> (models::Cancel, impl FnMut(models::Progress)) {
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let signal = cancel.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            signal.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    });
    let mut last = std::time::Instant::now() - Duration::from_secs(1);
    (cancel, move |p: models::Progress| {
        if last.elapsed() >= Duration::from_secs(1) || p.received == p.total {
            eprintln!(
                "{} · {} / {}",
                display_text(&p.stage),
                models::human_bytes(p.received),
                models::human_bytes(p.total)
            );
            last = std::time::Instant::now();
        }
    })
}

async fn model_action(root: &std::path::Path, command: ModelAction) -> Result<()> {
    let (artifact, select) = match command {
        ModelAction::Remove { id, delete_weights } => {
            models::remove(root, &id, delete_weights)?;
            println!("Model removed from library; imported originals were preserved.");
            return Ok(());
        }
        ModelAction::Relocate { destination } => {
            let (cancel, _) = transfer_status();
            println!(
                "{}",
                serde_json::to_string_pretty(&models::relocate(root, &destination, cancel).await?)?
            );
            return Ok(());
        }
        ModelAction::Cache => {
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &json!({"cache":models::cache_root(root)?,"receipt":std::fs::read_to_string(root.join("cache-relocation.json")).ok()})
                )?
            );
            return Ok(());
        }
        ModelAction::Local => {
            println!("{}", serde_json::to_string_pretty(&models::library(root)?)?);
            return Ok(());
        }
        ModelAction::Search { query, all } => {
            for model in models::search(&query, !all).await? {
                println!(
                    "{}  ({} downloads)",
                    display_text(&model.id),
                    model.downloads
                );
            }
            return Ok(());
        }
        ModelAction::Files { repository } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&models::files(&repository).await?)?
            );
            return Ok(());
        }
        ModelAction::Import {
            path,
            uncensored,
            select,
        } => {
            let (cancel, progress) = transfer_status();
            (
                models::import(root, &path, uncensored, cancel, progress).await?,
                select,
            )
        }
        ModelAction::Download {
            repository,
            file,
            select,
        } => {
            let metadata = models::files(&repository).await?.into_iter()
                .find(|f| f.filename == file)
                .context("That complete GGUF filename is not available; use `alt models files REPOSITORY`")?;
            eprintln!(
                "License declared by publisher: {}",
                display_text(&metadata.license)
            );
            let (cancel, progress) = transfer_status();
            (
                models::download(root, metadata, cancel, progress).await?,
                select,
            )
        }
    };
    if select {
        let preferences = Preferences::load(root)?;
        let mut config = Config::load(root)?;
        let name = format!("Local: {}", artifact.name);
        config.upsert(
            name.clone(),
            Profile {
                provider: Provider::Openai,
                endpoint: "http://127.0.0.1:8080/v1".into(),
                model: artifact.name.clone(),
                context_tokens: preferences.context_tokens,
                max_turns: preferences.max_turns,
                uncensored: artifact.uncensored_claim,
                api_key_env: None,
                local_model: Some(artifact.id.clone()),
            },
        )?;
        config.default_profile = name;
        config.save(root)?;
    }
    println!("{}", serde_json::to_string_pretty(&artifact)?);
    Ok(())
}

#[derive(Clone, Copy)]
enum Approval {
    Deny,
    Allow,
    Ask,
}

async fn headless(
    engine: &mut Engine,
    store: &Store,
    session: &Session,
    prompt: &str,
    allow: Approval,
    json_output: bool,
    timeout: u64,
) -> Result<()> {
    ensure!(timeout > 0, "Timeout must be positive");
    eprintln!("Session: {}", session.id);
    let user = json!({"type":"user","text":prompt});
    store.append(&session.id, &user)?;
    if json_output {
        println!("{}", json!({"type":"session","session":session}));
        println!("{user}");
    }
    let brief = store.brief(std::path::Path::new(&session.cwd))?;
    let mut response = engine
        .prompt_with_brief(&session.engine_id, prompt, &brief)
        .await?;
    let deadline = tokio::time::sleep(Duration::from_secs(timeout));
    tokio::pin!(deadline);
    let mut interrupted = false;
    let shutdown = alt_cli::process::shutdown_signal();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            result = &mut response => {
                let result = response_value(result)?;
                // Response and notification channels are separate; drain all
                // preceding notifications before returning the final response.
                while let Some(event) = engine.try_event() {
                    handle_event(engine, store, session, event, if interrupted {Approval::Deny} else {allow}, json_output).await?;
                }
                if let Some(verification) = engine.verification()? {
                    let note = if verification.complete { "All required checks passed on current files. Review their coverage." } else if verification.requirements.is_empty() { "Verification is not configured. Choose required checks in Task; a model response does not establish completion." } else { "Required checks are not complete on current files. Inspect Task verification before considering this finished." };
                    let evidence = json!({"type":"verification","data":verification,"note":note});
                    store.append(&session.id, &evidence)?;
                    if json_output { println!("{evidence}"); } else { println!("\n\nAlt verification: {note}"); }
                }
                let event = json!({"type":"turn_end","data":result});
                store.append(&session.id, &event)?;
                if json_output { println!("{event}"); } else { println!(); }
                if interrupted { bail!("Turn cancelled"); }
                return Ok(());
            }
            event = engine.next_event() => {
                let event = event.context("Engine event stream closed")?;
                handle_event(engine, store, session, event, if interrupted {Approval::Deny} else {allow}, json_output).await?;
            }
            _ = &mut shutdown, if !interrupted => {
                engine.cancel(&session.engine_id).await?;
                interrupted = true;
                deadline.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(5));
            }
            _ = &mut deadline => {
                if interrupted { bail!("Engine did not stop within 5 seconds; shutting it down"); }
                engine.cancel(&session.engine_id).await?;
                interrupted = true;
                deadline.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(5));
            }
        }
    }
}

async fn handle_event(
    engine: &mut Engine,
    store: &Store,
    session: &Session,
    event: Event,
    allow: Approval,
    json_output: bool,
) -> Result<()> {
    let record = tui::record(&event);
    store.append(&session.id, &record)?;
    if json_output {
        println!("{record}");
    }
    match event {
        Event::Update(params) if !json_output && params["sessionId"] == session.engine_id => {
            if let Some(text) = update_text(&params) {
                print!("{}", display_text(&text));
                std::io::stdout().flush()?;
            }
        }
        Event::Permission { id, params } => {
            let allow = if params["sessionId"] != session.engine_id {
                false
            } else {
                match allow {
                    Approval::Deny => false,
                    Approval::Allow => true,
                    Approval::Ask => {
                        println!(
                            "\nTool request: {}\n{}\nAllow once? [y/N]",
                            display_text(
                                params["toolCall"]["title"]
                                    .as_str()
                                    .unwrap_or("External action")
                            ),
                            display_text(&params["toolCall"]["rawInput"].to_string())
                        );
                        tokio::task::spawn_blocking(|| {
                            let mut line = String::new();
                            std::io::stdin()
                                .read_line(&mut line)
                                .map(|_| matches!(line.trim().to_lowercase().as_str(), "y" | "yes"))
                        })
                        .await??
                    }
                }
            };
            engine.permission(id.clone(), &params, allow).await?;
            let decision = json!({"type":"permission_decision","id":id,"allow":allow});
            store.append(&session.id, &decision)?;
            if json_output {
                println!("{decision}");
            } else {
                eprintln!(
                    "[tool permission: {}]",
                    if allow {
                        "allowed once"
                    } else {
                        "denied; use TUI or --allow-tools"
                    }
                );
            }
        }
        Event::Disconnected(reason) => bail!("{reason}"),
        _ => {}
    }
    Ok(())
}

async fn task_action(
    root: &std::path::Path,
    action: TaskAction,
    access: Option<alt_cli::project::Policy>,
) -> Result<()> {
    use alt_cli::project::{CheckSpec, Project};
    let cwd = std::env::current_dir()?;
    let mut p = Project::open(root, &cwd)?;
    let task = p
        .latest_task()?
        .unwrap_or_else(|| format!("manual-{}", uuid::Uuid::new_v4()));
    if p.task(&task).is_err() {
        p.start_task(&task, "Manual project checks and changes")?;
    }
    let value = match action {
        TaskAction::Contract {
            name,
            kind,
            format,
            report,
            assertion,
        } => {
            let mut spec = p
                .checks()?
                .into_iter()
                .find(|s| s.name == name)
                .context("Configure the command first")?;
            ensure!(
                format.is_some() == report.is_some(),
                "Supply both --format and --report"
            );
            spec.contract = alt_cli::verification::Contract {
                kind,
                report: format
                    .zip(report)
                    .map(|(format, path)| alt_cli::verification::ReportSpec { format, path }),
                assertion: assertion
                    .as_deref()
                    .map(alt_cli::verification::Contract::pin)
                    .transpose()?,
            };
            if let Some(assertion) = &spec.contract.assertion {
                // Pin the script already named by the user; never substitute an unrelated command.
                for arg in spec.argv.iter_mut().take(2) {
                    if std::path::Path::new(arg).canonicalize().ok().as_ref()
                        == Some(&assertion.path)
                    {
                        *arg = "{assertion}".into();
                    }
                }
            }
            p.set_check(&spec)?;
            json!(spec)
        }
        TaskAction::Require {
            name,
            check,
            description,
        } => {
            p.set_requirement(&alt_cli::project_services::Requirement {
                name,
                check_name: check,
                description,
            })?;
            json!(p.verification()?)
        }
        TaskAction::Unrequire { name } => {
            p.remove_requirement(&name)?;
            json!(p.verification()?)
        }
        TaskAction::Verify { run } => {
            if run {
                let requirements = p.requirements()?;
                drop(p);
                let (cancel, _) = transfer_status();
                for requirement in requirements {
                    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }
                    alt_cli::sandbox::run(
                        root.into(),
                        cwd.clone(),
                        task.clone(),
                        requirement.check_name,
                        access.unwrap_or(Preferences::load(root)?.access_policy),
                        cancel.clone(),
                    )
                    .await?;
                }
                p = Project::open(root, &cwd)?;
            }
            let result = p.verification()?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            ensure!(
                result.complete,
                "Required checks are incomplete; inspect the verification report"
            );
            return Ok(());
        }
        TaskAction::Pin { text } => json!({"id":p.pin(&text)?}),
        TaskAction::Unpin { id } => {
            p.unpin(&id)?;
            json!({"removed":true})
        }
        TaskAction::Context => p.context_view(&task)?,
        TaskAction::Map => p.repository_map()?,
        TaskAction::Index => json!(p.index_incremental(None)?),
        TaskAction::Status => json!({"task":p.task(&task)?,"verification":p.verification()?}),
        TaskAction::Changes => json!(p.changes(None)?),
        TaskAction::Diff { id } => {
            println!("{}", display_text(&p.change(&id)?.diff()));
            return Ok(());
        }
        TaskAction::Undo { id } => json!(p.undo(&id)?),
        TaskAction::UndoAll { task } => json!(p.undo_task(&task)?),
        TaskAction::Checks => {
            json!({"configured":p.checks()?,"suggested":alt_cli::sandbox::suggested(&cwd)})
        }
        TaskAction::ConfigureCheck {
            name,
            timeout,
            argv,
        } => {
            let spec = CheckSpec {
                name,
                argv,
                timeout_secs: timeout,
                contract: Default::default(),
            };
            p.set_check(&spec)?;
            json!(spec)
        }
        TaskAction::Check { name } => {
            drop(p);
            let policy = access.unwrap_or(Preferences::load(root)?.access_policy);
            let (cancel, _) = transfer_status();
            let result =
                alt_cli::sandbox::run(root.into(), cwd, task, name, policy, cancel).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            ensure!(
                result.exit_code == Some(0)
                    && result.error.is_none()
                    && !result.timed_out
                    && !result.cancelled,
                "Check did not pass; evidence saved"
            );
            return Ok(());
        }
        TaskAction::Memory { query } => {
            let memory = p.memory(&task, query.as_deref().unwrap_or("project"), 16000)?;
            println!("{}", display_text(&memory));
            return Ok(());
        }
        TaskAction::Remember { text } => {
            p.note(&task, "decision", &text, "user")?;
            json!({"saved":true})
        }
        TaskAction::Export => p.export()?,
    };
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

async fn job_action(
    root: &std::path::Path,
    action: JobAction,
    access: Option<alt_cli::project::Policy>,
) -> Result<()> {
    use alt_cli::jobs;
    if matches!(
        action,
        JobAction::Start { .. }
            | JobAction::Restart { .. }
            | JobAction::Input { .. }
            | JobAction::Attach { .. }
    ) {
        ensure!(
            access.unwrap_or(Preferences::load(root)?.access_policy)
                == alt_cli::project::Policy::Trusted,
            "Jobs execute with host permissions. Choose Full access in Settings or use --access trusted"
        );
    }
    let wait_for_completion = matches!(&action, JobAction::Start { wait: true, .. });
    let value = match action {
        JobAction::List => json!(jobs::list(root)?),
        JobAction::Start {
            command,
            name,
            timeout,
            wait,
        } => {
            let mut r = jobs::start(
                root,
                jobs::Spec {
                    name,
                    command,
                    cwd: std::env::current_dir()?,
                    keep: !wait,
                    timeout_secs: timeout,
                    rows: 24,
                    cols: 100,
                },
            )
            .await?;
            if wait {
                let shutdown = alt_cli::process::shutdown_signal();
                tokio::pin!(shutdown);
                loop {
                    tokio::select! {_=&mut shutdown=>{r=jobs::stop(root,&r.id).await?;break;},_=tokio::time::sleep(Duration::from_millis(100))=>{r=jobs::record(root,&r.id)?;if r.status!="running"{break;}}}
                }
            }
            json!(r)
        }
        JobAction::Stop { id } => json!(jobs::stop(root, &id).await?),
        JobAction::Restart { id } => json!(jobs::restart(root, &id).await?),
        JobAction::Logs { id } => {
            println!("{}", jobs::output(root, &id)?);
            return Ok(());
        }
        JobAction::Attach { id } => return jobs::attach(root, &id).await,
        JobAction::Input { id, text } => {
            jobs::request(root, &id, json!({"action":"input","text":text})).await?
        }
        JobAction::Resize { id, cols, rows } => {
            jobs::request(
                root,
                &id,
                json!({"action":"resize","cols":cols,"rows":rows}),
            )
            .await?
        }
        JobAction::Health { id, url } => jobs::health(root, &id, &url).await?,
    };
    println!("{}", serde_json::to_string_pretty(&value)?);
    ensure!(value["healthy"] != false, "Job health check failed");
    ensure!(
        !wait_for_completion || value["status"] == "completed",
        "Job did not complete successfully"
    );
    Ok(())
}
