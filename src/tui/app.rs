use super::{input::Editor, view};
use crate::{
    config::{Config, Preferences, Profile, Provider, private_dir},
    display_text,
    engine::Event,
    models::{self, Artifact, Cancel, HubModel, ModelFile, Progress},
    runtime::{self, Component, Hardware},
    store::{Session, SessionSummary, Store},
    workspace::{self, Workspace},
};
use anyhow::{Context, Result, ensure};
use crossterm::{
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event as TerminalEvent, EventStream, KeyEventKind,
    },
    execute,
};
use futures_util::StreamExt;
use ratatui::layout::Rect;
use serde_json::Value;
use std::{
    collections::VecDeque,
    io::IsTerminal,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Chat,
    Models,
    Connections,
    Sessions,
    Settings,
    Help,
    Task,
    Files,
    Jobs,
    Context,
}
impl Page {
    pub const ALL: [Self; 11] = [
        Self::Home,
        Self::Chat,
        Self::Models,
        Self::Connections,
        Self::Sessions,
        Self::Settings,
        Self::Help,
        Self::Task,
        Self::Files,
        Self::Jobs,
        Self::Context,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Chat => "Workspace",
            Self::Models => "Models",
            Self::Connections => "Connections",
            Self::Sessions => "Conversations",
            Self::Settings => "Settings",
            Self::Help => "Help",
            Self::Task => "Task progress",
            Self::Files => "Project files",
            Self::Jobs => "Terminal jobs",
            Self::Context => "Context & memory",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Sidebar,
    TopTabs,
    BottomTabs,
    Split,
}
impl LayoutMode {
    pub const ALL: [Self; 4] = [Self::Sidebar, Self::TopTabs, Self::BottomTabs, Self::Split];
    pub fn label(self) -> &'static str {
        match self {
            Self::Sidebar => "Sidebar",
            Self::TopTabs => "Top tabs",
            Self::BottomTabs => "Bottom tabs",
            Self::Split => "Split (sidebar + activity)",
        }
    }
    pub fn from_saved(name: &str) -> Self {
        Self::ALL
            .iter()
            .find(|mode| {
                mode.label().split_whitespace().next()
                    == Some(name.split_whitespace().next().unwrap_or(""))
            })
            .copied()
            .unwrap_or(Self::Sidebar)
    }
    pub fn saved_name(self) -> &'static str {
        match self {
            Self::Sidebar => "sidebar",
            Self::TopTabs => "top",
            Self::BottomTabs => "bottom",
            Self::Split => "split",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Hit {
    Nav(Page),
    Home(usize),
    Row(usize),
    Button(&'static str),
    ModelTab(usize),
    SettingsTab(usize),
    Field(usize),
    Choice(usize),
    Composer,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: &'static str,
    pub text: String,
}
#[derive(Debug, Clone)]
pub struct Tool {
    pub id: String,
    pub title: String,
    pub status: String,
    pub input: Value,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ConnectionDraft {
    pub name: String,
    pub profile: Profile,
    pub original: Option<String>,
}
#[derive(Debug, Clone)]
pub struct ConnectionForm {
    pub fields: [Editor; 3],
    pub provider: Provider,
    pub focus: usize,
    pub original: Option<String>,
    /// Thinking dialect from a provider preset (A03); Auto otherwise.
    pub dialect: crate::inference::Dialect,
}
#[derive(Debug, Clone, Copy)]
pub enum BrowserKind {
    Project,
    Model,
    Engine,
    Runtime,
}
#[derive(Debug, Clone)]
pub enum InputAction {
    Manager {
        action: String,
        state: Value,
    },
    FileFilter,
    FileLine(String),
    FileText(String),
    NewFile,
    JobCommand,
    JobTimeout {
        command: String,
        keep: bool,
    },
    JobHealth(String),
    PinRequirement,
    EditFile {
        path: String,
        original: String,
        task: String,
        new: bool,
    },
    CheckCommand,
    MemorySearch,
    TaskNote,
    SearchHub,
    DirectRepo,
    ImportPath,
    Rename(String),
    Brief,
    EnginePath,
    RuntimePath,
    ProjectPath,
    NewProjectPath,
    ManualModel(Box<ConnectionDraft>),
}
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    Manager { action: String, state: Value },
    ApplyFile(String),
    Undo(String),
    UndoTask(String),
    Access(crate::project::Policy),
    Evaluate,
    Download(ModelFile),
    Install(Component),
    Archive(String, bool),
    RemoveConnection(String),
    Quit,
}
#[derive(Debug, Clone)]
pub enum MenuAction {
    NewPractice,
    Manager {
        action: String,
        state: Value,
    },
    StartJob {
        command: String,
        keep: bool,
    },
    RequireCheck(String),
    Unpin(String),
    RunCheck(String),
    ConfigureCheck(crate::project::CheckSpec),
    CustomCheck,
    Access(crate::project::Policy),
    HardwareChoice(u32),
    Preset(usize),
    /// A03 provider preset by index into `presets::PRESETS`.
    ProviderPreset(usize),
    Template(usize),
    Page(Page),
    New,
    Brief,
    Project,
    CurrentFolder,
    RecentProject(usize),
    NewFolder,
    Connect,
    /// A settings row by its global index; the palette reaches every setting.
    Setting(usize),
}

#[derive(Debug, Clone)]
pub enum Dialog {
    Menu {
        title: String,
        description: String,
        items: Vec<(String, String, MenuAction)>,
        selected: usize,
    },
    Connection(ConnectionForm),
    PickModel {
        draft: Box<ConnectionDraft>,
        models: Vec<models::ServerModel>,
        selected: usize,
        /// Typing filters long hosted catalogs; `selected` indexes the filtered rows.
        query: Editor,
    },
    Palette {
        query: Editor,
        selected: usize,
    },
    Input {
        title: String,
        hint: String,
        editor: Editor,
        action: InputAction,
        multiline: bool,
    },
    Browser {
        kind: BrowserKind,
        path: PathBuf,
        entries: Vec<PathBuf>,
        selected: usize,
    },
    Confirm {
        title: String,
        body: String,
        action: ConfirmAction,
        selected: usize,
    },
    Notice {
        title: String,
        body: String,
        scroll: u16,
    },
    ToolDetails {
        tool: Tool,
        scroll: u16,
    },
}

pub enum JobResult {
    Practice(crate::practice::Practice),
    Resume(Session),
    PreparedWorkspace(Box<workspace::Options>, Vec<Value>, String),
    Brief(String),
    Dialog(Dialog),
    Context(String),
    Sessions(String, bool, Vec<SessionSummary>),
    Jobs(Vec<crate::jobs::Record>),
    Saved(String),
    Manager(String, Value),
    ProjectFiles(Vec<String>),
    ManagedJob(crate::jobs::Record),
    Task(Box<super::task::TaskView>),
    Check(crate::project::CheckResult),
    Notice(String, String),
    Hardware(crate::hardware::Report),
    Inventory(String, Vec<models::ServerModel>),
    Search(Vec<HubModel>),
    Files(Vec<ModelFile>),
    Tested(ConnectionDraft, Vec<models::ServerModel>),
    Artifact(Artifact),
    Installed(Component, PathBuf),
}
pub enum JobEvent {
    Progress(Progress),
    Finished(Box<Result<JobResult, String>>),
}
pub struct ScopedJobEvent {
    pub id: uuid::Uuid,
    pub generation: u64,
    pub event: JobEvent,
}
pub struct Job {
    pub id: uuid::Uuid,
    pub label: String,
    pub progress: Progress,
    pub cancel: Cancel,
    pub task: tokio::task::JoinHandle<()>,
    pub started: Instant,
    pub recovery: Option<Dialog>,
    /// Only disposable view refreshes may yield to an explicit user operation.
    pub replaceable: bool,
}

pub struct App {
    pub workbench: super::workbench::Workbench,
    pub task_view: super::task::TaskView,
    pub change_selected: usize,
    pub root: PathBuf,
    pub requested_engine: PathBuf,
    pub config: Config,
    pub preferences: Preferences,
    pub hardware: Hardware,
    pub page: Page,
    pub nav_focus: bool,
    pub nav_index: usize,
    pub hits: Vec<(Rect, Hit)>,
    pub dialog: Option<Dialog>,
    pub status: String,
    pub status_error: bool,
    pub recovery_dialog: Option<Box<Dialog>>,
    pub project_missing: bool,
    pub theme: &'static crate::tui::theme::Theme,
    /// Monotonic render counter.
    pub frame: u64,
    /// Animation clock and transition state (M01).
    pub motion: super::motion::Motion,
    pub layout: LayoutMode,
    pub home_selected: usize,
    pub connection_selected: usize,
    pub session_selected: usize,
    pub settings_selected: usize,
    pub settings_tab: usize,
    pub sessions: Vec<SessionSummary>,
    pub session_search: Editor,
    pub search_focus: bool,
    pub archived: bool,
    pub model_tab: usize,
    pub model_selected: usize,
    pub artifacts: Vec<Artifact>,
    pub server_models: Vec<models::ServerModel>,
    pub server_source: Option<String>,
    pub hub_models: Vec<HubModel>,
    pub hub_files: Vec<ModelFile>,
    pub hub_query: String,
    pub variants_only: bool,
    pub workspace: Option<Workspace>,
    pub session: Option<Session>,
    pub connected: bool,
    pub connecting: bool,
    pub busy: bool,
    pub cancelling: bool,
    pub turn_started: Option<Instant>,
    pub turn_tools: usize,
    pub messages: VecDeque<Message>,
    pub composer: Editor,
    pub pending_prompt: Option<String>,
    pub tools: Vec<Tool>,
    pub tool_selected: usize,
    pub tools_focus: bool,
    pub chat_scroll: u16,
    pub help_scroll: u16,
    pub permissions: VecDeque<(Value, Value)>,
    pub permission_selected: usize,
    pub permission_scroll: u16,
    pub trust_session: bool,
    /// Separate explicit opt-in for terminal/external (host-scope) actions.
    pub trust_host: bool,
    pub context_used: Option<u64>,
    pub context_size: Option<u64>,
    pub inference_status: Option<crate::inference::Status>,
    /// A reasoning-only cutoff arrived; offer recovery when the turn ends.
    pub cutoff_pending: bool,
    pub brief: String,
    pub job: Option<Job>,
    pub job_tx: mpsc::Sender<ScopedJobEvent>,
    pub job_rx: mpsc::Receiver<ScopedJobEvent>,
    pub quit: bool,
    pub generation: u64,
    pub pending_refresh: Option<Page>,
    pub pending_sessions: bool,
    pub pending_task_notice: Option<bool>,
}

pub const TEMPLATES: [(&str, &str, &str); 5] = [
    (
        "Understand this project",
        "Get a plain-language tour before changing anything.",
        "Inspect this project and explain what it does, how its parts fit together, and how to run it. Use plain language. Do not change files yet.",
    ),
    (
        "Build or change something",
        "Describe the result you want; review changes as you go.",
        "I want to build or change: [describe the result]. First inspect the project, explain a short plan in plain language, and ask about any essential missing details. Then make focused changes and verify them.",
    ),
    (
        "Investigate a problem",
        "Find the cause using files, errors, and reproducible checks.",
        "Help me investigate this problem: [describe what happened and what you expected]. Inspect the relevant files, reproduce the issue where practical, explain the cause, and propose a focused fix.",
    ),
    (
        "Check that it works",
        "Discover the existing checks and report actual results.",
        "Inspect this project's setup and existing checks. Run the appropriate tests or checks, report actual results and errors, and explain what needs attention. Do not claim success without tool evidence.",
    ),
    (
        "Review software security",
        "Inspect the selected project and document evidence.",
        "Review this project for security weaknesses within the local project scope. Inspect the code and configuration, verify suspected issues with bounded local checks, and give evidence, impact, and practical fixes. Do not contact external targets unless I explicitly name and authorize them.",
    ),
];

impl App {
    pub fn load(root: PathBuf, requested_engine: PathBuf, profile: Option<String>) -> Result<Self> {
        private_dir(&root)?;
        let root = root.canonicalize()?;
        let mut recovery = Vec::new();
        let config = recover_settings(&root, "config.toml", Config::load, &mut recovery)?;
        let preferences =
            recover_settings(&root, "preferences.toml", Preferences::load, &mut recovery)?;
        let store = Store::open(&root)?;
        let sessions = store.summaries("", false)?;
        let brief = store.brief(&preferences.project)?;
        let artifacts = match models::library(&root) {
            Ok(artifacts) => artifacts,
            Err(error) => {
                recovery.push(format!("Model library needs attention: {error:#}"));
                Vec::new()
            }
        };
        let (tx, rx) = mpsc::channel(32);
        let theme = crate::tui::theme::by_name(preferences.theme.as_str());
        let motion = super::motion::Motion::new(super::motion::Level::resolve(&preferences.motion));
        let layout = LayoutMode::from_saved(&preferences.layout);
        let mut app = Self {
            workbench: Default::default(),
            root,
            requested_engine,
            config,
            preferences,
            hardware: Hardware::default(),
            page: Page::Home,
            nav_focus: false,
            nav_index: 0,
            hits: Vec::new(),
            dialog: None,
            status: "Welcome. Choose a model and project, then describe what you want to do."
                .into(),
            status_error: false,
            recovery_dialog: None,
            project_missing: false,
            theme,
            frame: 0,
            motion,
            layout,
            home_selected: 0,
            connection_selected: 0,
            session_selected: 0,
            settings_selected: 0,
            settings_tab: 0,
            task_view: Default::default(),
            change_selected: 0,
            sessions,
            session_search: Editor::default(),
            search_focus: false,
            archived: false,
            model_tab: 0,
            model_selected: 0,
            artifacts,
            server_models: Vec::new(),
            server_source: None,
            hub_models: Vec::new(),
            hub_files: Vec::new(),
            hub_query: "Qwen3 4B heretic".into(),
            variants_only: true,
            workspace: None,
            session: None,
            connected: false,
            connecting: false,
            busy: false,
            cancelling: false,
            turn_started: None,
            turn_tools: 0,
            messages: VecDeque::new(),
            composer: Editor::default(),
            pending_prompt: None,
            tools: Vec::new(),
            tool_selected: 0,
            tools_focus: false,
            chat_scroll: 0,
            help_scroll: 0,
            permissions: VecDeque::new(),
            permission_selected: 0,
            permission_scroll: 0,
            trust_session: false,
            trust_host: false,
            context_used: None,
            context_size: None,
            inference_status: None,
            cutoff_pending: false,
            brief,
            job: None,
            job_tx: tx,
            job_rx: rx,
            quit: false,
            generation: 0,
            pending_refresh: None,
            pending_sessions: false,
            pending_task_notice: None,
        };
        if let Some(name) = profile {
            ensure!(
                app.config.profiles.contains_key(&name),
                "Unknown connection: {name}"
            );
            app.config.default_profile = name;
        }
        if !recovery.is_empty() {
            app.error(format!("Some saved settings or model records need attention. The details below explain what was preserved and how to recover. Conversations and project files are unchanged.\n\n{}", recovery.join("\n\n")));
        }
        // N02: detect a dead saved project before the first frame so no
        // launch ever opens on a workspace that cannot be used.
        if !app.preferences.project.is_dir() {
            app.project_missing = true;
            app.open_project_chooser();
        } else if let Some(warning) = crate::project::broad_root_warning(&app.preferences.project) {
            app.error(warning);
        }
        Ok(app)
    }

    pub fn current_profile(&self) -> Option<(&str, &Profile)> {
        self.config.profile(None).ok()
    }

    pub const SETTINGS_TABS: [&str; 5] =
        ["Appearance", "Models", "Tools & access", "Runtime", "Data"];
    /// Global setting indexes shown in each tab, in order. Indexes match
    /// `setting_action` and `settings_all_items` exactly.
    pub fn settings_tab_indexes(tab: usize) -> &'static [usize] {
        const TABS: [&[usize]; 5] = [
            &[2, 3, 4, 23],
            &[0, 19, 21, 20, 10, 11, 16],
            &[8, 17, 12],
            &[1, 5, 6, 22, 9, 15, 18],
            &[7, 13, 14],
        ];
        TABS[tab.min(TABS.len() - 1)]
    }
    /// All settings rows in the exact order setting_action indexes expect.
    pub fn settings_all_items(&self, context: u32, budget: u32) -> Vec<(String, String)> {
        vec![
            (
                format!(
                    "Context: {} tokens · {}",
                    context,
                    match context {
                        4096 => "Economy",
                        8192 => "Balanced",
                        16384 => "Extended",
                        _ => "Large",
                    }
                ),
                "More context uses more memory. Cycle: 4K → 8K → 16K → 32K.".into(),
            ),
            (
                format!("Task budget: {budget} steps"),
                "Limits tool/model turns before asking you to continue. Cycle: 6 → 12 → 24."
                    .into(),
            ),
            (
                format!("Theme: {}", self.theme.name),
                "Five looks including the neon default; Alt redraws immediately.".into(),
            ),
            (
                format!("Layout: {}", self.layout.label()),
                "Ctrl+L also cycles: sidebar, top tabs, bottom tabs, split.".into(),
            ),
            (
                format!(
                    "Mouse: {}",
                    if self.preferences.mouse {
                        "enabled"
                    } else {
                        "disabled"
                    }
                ),
                "Keyboard navigation is always available.".into(),
            ),
            (
                "Agent engine executable".into(),
                self.preferences
                    .engine_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "Auto-detect Goose; install from Home if missing".into()),
            ),
            (
                "Local model runtime executable".into(),
                self.preferences
                    .runtime_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| {
                        "Auto-detect llama-server; install from Models if missing".into()
                    }),
            ),
            (
                "Project brief".into(),
                "Edit the goals and decisions sent with each new message.".into(),
            ),
            (
                format!(
                    "Access: {} · {} configured check(s)",
                    self.preferences.access_policy.label(),
                    self.task_view.checks.len()
                ),
                "Choose Review only, Guided changes, or Full access terminal.".into(),
            ),
            (
                "Fit Alt to this computer".into(),
                "Inspect hardware and choose a low-memory, balanced or larger context profile."
                    .into(),
            ),
            (
                "Check this model's tool use".into(),
                "A short live probe of the exact selected uncensored model. Saves its evidence."
                    .into(),
            ),
            (
                format!(
                    "Selected model is uncensored/abliterated: {}",
                    if self.current_profile().is_some_and(|(_, p)| p.uncensored) {
                        "claimed"
                    } else {
                        "not marked"
                    }
                ),
                "Enter records/removes your publisher claim, enabling the explicit live evaluation."
                    .into(),
            ),
            (
                "Tools and workflows".into(),
                "Build, Git, HTTP, browser, security and selected external MCP tools.".into(),
            ),
            (
                "Storage and recovery".into(),
                "Backup, restore, disk usage, retention and redacted diagnostics.".into(),
            ),
            (
                "Model files and cache".into(),
                "Private Hub access, verified relocation and removing managed weights.".into(),
            ),
            (
                "Model and runtime settings".into(),
                "Output allocation, skills, workflow, effort and CPU/GPU settings.".into(),
            ),
            (
                "Benchmark selected model".into(),
                "Measure this computer; exports exact settings, timings and runtime memory.".into(),
            ),
            (
                format!("Tool focus: {}", self.preferences.tool_profile.label()),
                "Choose all tools, inspection, coding or terminal schemas; access mode stays your choice.".into(),
            ),
            (
                "Qualify model contexts and restart".into(),
                "Measure generation, memory, cancellation cleanup and restart for 2K, 4K and 8K."
                    .into(),
            ),
            (
                format!(
                    "Thinking: {}",
                    self.current_profile()
                        .map(|(_, p)| p.effective_inference().effective_thinking().label())
                        .unwrap_or_else(|| "choose a model first".into())
                ),
                "Off, low, medium, high or server default; sent in fields this server understands."
                    .into(),
            ),
            (
                "Advanced parameters".into(),
                "Penalties, seed, stop, extra request JSON, Ollama context and llama-server options."
                    .into(),
            ),
            (
                "Probe this model".into(),
                "One short request; suggests thinking, output allowance, compact tools and context."
                    .into(),
            ),
            (
                "GPU runtime (CUDA 12)".into(),
                "Detect an NVIDIA GPU and install or select a CUDA 12 llama-server; CPU stays default."
                    .into(),
            ),
            (
                format!("Motion: {}", self.motion.level.label()),
                "Full, Reduced (fades only) or Off. ALT_MOTION overrides; the Linux console is always Off."
                    .into(),
            ),
        ]
    }

    /// Every command reachable from the Ctrl+P palette. Keyboard and mouse
    /// share this one list, so nothing exists outside it.
    pub fn palette_items(&self) -> Vec<(String, String, MenuAction)> {
        let mut items: Vec<(String, String, MenuAction)> = Page::ALL
            .iter()
            .map(|p| {
                (
                    p.name().to_string(),
                    "Open page".into(),
                    MenuAction::Page(*p),
                )
            })
            .collect();
        items.extend([
            (
                "New conversation".into(),
                "Keep previous conversations saved".into(),
                MenuAction::New,
            ),
            (
                "Connect a model…".into(),
                "Guided setup for Ollama, LM Studio, servers or files".into(),
                MenuAction::Connect,
            ),
            (
                "Connect a hosted API…".into(),
                "NVIDIA NIM, OpenRouter, Groq, Together, DeepSeek, Mistral, Gemini, Cerebras"
                    .into(),
                MenuAction::Preset(5),
            ),
            (
                "Choose project folder…".into(),
                "Browse this computer".into(),
                MenuAction::Project,
            ),
            (
                "Create a project folder…".into(),
                "Make a folder and open it".into(),
                MenuAction::NewFolder,
            ),
            (
                "Try a practice project".into(),
                "Fresh example; learns repair, checks and undo".into(),
                MenuAction::NewPractice,
            ),
            (
                "Edit project brief".into(),
                "Goals and decisions sent with new messages".into(),
                MenuAction::Brief,
            ),
            (
                "Appearance: themes".into(),
                "Switch the whole look; redraws immediately".into(),
                MenuAction::Manager {
                    action: "appearance".into(),
                    state: serde_json::json!({}),
                },
            ),
            (
                "Tools and workflows".into(),
                "Packs, MCP connections, findings".into(),
                MenuAction::Manager {
                    action: "manage-tools".into(),
                    state: serde_json::json!({}),
                },
            ),
            (
                "Bundled skills on/off".into(),
                "Disabled skills are never offered to the model".into(),
                MenuAction::Manager {
                    action: "skills-manager".into(),
                    state: serde_json::json!({}),
                },
            ),
            (
                "Storage and recovery".into(),
                "Backups, disk use, retention".into(),
                MenuAction::Manager {
                    action: "manage-storage".into(),
                    state: serde_json::json!({}),
                },
            ),
            (
                "Model files and cache".into(),
                "Managed weights, relocation, removal".into(),
                MenuAction::Manager {
                    action: "manage-models".into(),
                    state: serde_json::json!({}),
                },
            ),
            (
                "Model and runtime settings".into(),
                "Allocation, skills, workflow, CPU/GPU".into(),
                MenuAction::Manager {
                    action: "runtime-settings".into(),
                    state: serde_json::json!({}),
                },
            ),
            (
                "Model thinking: off, low, medium, high".into(),
                "Reasoning control translated for this server".into(),
                MenuAction::Manager {
                    action: "inference-thinking".into(),
                    state: serde_json::json!({}),
                },
            ),
            (
                "GPU runtime (CUDA 12)".into(),
                "Detect an NVIDIA GPU; install or select a CUDA runtime on request".into(),
                MenuAction::Manager {
                    action: "cuda-runtime".into(),
                    state: serde_json::json!({}),
                },
            ),
            (
                "Probe this model".into(),
                "One short request; suggests thinking, output, tools and context".into(),
                MenuAction::Manager {
                    action: "probe-run".into(),
                    state: serde_json::json!({}),
                },
            ),
            (
                "Advanced parameters".into(),
                "Penalties, seed, stop, extra JSON, Ollama context, llama-server options".into(),
                MenuAction::Manager {
                    action: "params-menu".into(),
                    state: serde_json::json!({}),
                },
            ),
            (
                "Hosted output guard".into(),
                "Discard leaked-token garbage once and resend; retries for 429/5xx".into(),
                MenuAction::Manager {
                    action: "inference-guard".into(),
                    state: serde_json::json!({}),
                },
            ),
            (
                "Recover from a thinking cutoff".into(),
                "Resend with thinking off or a larger allowance".into(),
                MenuAction::Manager {
                    action: "inference-cutoff".into(),
                    state: serde_json::json!({}),
                },
            ),
            (
                "Inference allowance".into(),
                "Live request/token budgets".into(),
                MenuAction::Manager {
                    action: "inference-live".into(),
                    state: serde_json::json!({}),
                },
            ),
            (
                "Access: Review only".into(),
                "The model may inspect but not change".into(),
                MenuAction::Access(crate::project::Policy::ReviewOnly),
            ),
            (
                "Access: Guided changes".into(),
                "Reviewed edits with isolated checks".into(),
                MenuAction::Access(crate::project::Policy::Guided),
            ),
            (
                "Access: Full access".into(),
                "Any command, network and installs with your permissions".into(),
                MenuAction::Access(crate::project::Policy::Trusted),
            ),
        ]);
        // Every settings row, so nothing in Settings is unreachable from here.
        let context = self
            .current_profile()
            .map(|(_, p)| p.context_tokens)
            .unwrap_or(self.preferences.context_tokens);
        let budget = self
            .current_profile()
            .map(|(_, p)| p.max_turns)
            .unwrap_or(self.preferences.max_turns);
        for (index, (label, hint)) in self
            .settings_all_items(context, budget)
            .into_iter()
            .enumerate()
        {
            items.push((
                format!("Setting: {label}"),
                hint,
                MenuAction::Setting(index),
            ));
        }
        items
    }

    /// Case-insensitive contains-or-subsequence filter with a simple score;
    /// an empty query shows everything.
    pub fn palette_filtered(&self) -> Vec<(String, String, MenuAction)> {
        let query = self
            .dialog
            .as_ref()
            .and_then(|d| match d {
                Dialog::Palette { query, .. } => Some(query.text.trim().to_lowercase()),
                _ => None,
            })
            .unwrap_or_default();
        let mut scored: Vec<(i32, (String, String, MenuAction))> = self
            .palette_items()
            .into_iter()
            .map(|item| {
                let hay = format!("{} {}", item.0, item.1).to_lowercase();
                let score = if query.is_empty() {
                    1
                } else if hay.contains(&query) {
                    3
                } else {
                    let mut chars = hay.chars();
                    let subsequence = query.chars().all(|qc| chars.any(|hc| hc == qc));
                    if subsequence { 2 } else { 0 }
                };
                (score, item)
            })
            .filter(|(score, _)| *score > 0)
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.0.cmp(&b.1.0)));
        scored.into_iter().map(|(_, item)| item).collect()
    }
    pub fn engine_ready(&self) -> bool {
        runtime::find_engine(&self.root, &self.requested_engine, &self.preferences).is_some()
    }
    pub fn runtime_ready(&self) -> bool {
        runtime::find_runtime(&self.root, &self.preferences).is_some()
    }
    pub fn notify(&mut self, text: impl Into<String>) {
        self.status = text.into();
        self.status_error = false;
        let lowered = self.status.to_lowercase();
        let tone = if ["fail", "could not", "cannot", "error", "denied"]
            .iter()
            .any(|word| lowered.contains(word))
        {
            super::motion::Tone::Bad
        } else if [
            "saved",
            "connected",
            "changed",
            "applied",
            "completed",
            "passed",
            "ready",
        ]
        .iter()
        .any(|word| lowered.contains(word))
        {
            super::motion::Tone::Good
        } else {
            super::motion::Tone::Info
        };
        let text = self.status.clone();
        self.motion.toast(&text, tone);
    }
    /// Status-line progress without a floating notice (frequent stage updates).
    pub fn progress_status(&mut self, text: impl Into<String>) {
        self.status = text.into();
        self.status_error = false;
    }
    /// N08: Ctrl+L cycles Sidebar → Top tabs → Bottom tabs → Split and back.
    pub fn cycle_layout(&mut self) -> Result<()> {
        self.layout = LayoutMode::ALL[(LayoutMode::ALL
            .iter()
            .position(|m| *m == self.layout)
            .unwrap_or(0)
            + 1)
            % LayoutMode::ALL.len()];
        self.preferences.layout = self.layout.saved_name().to_owned();
        self.preferences.save(&self.root)?;
        self.notify(format!(
            "Layout: {}. Ctrl+L switches again.",
            self.layout.label()
        ));
        Ok(())
    }
    pub fn error(&mut self, error: impl std::fmt::Display) {
        let mut text = display_text(&error.to_string());
        let lowered = text.to_lowercase();
        let next = if lowered.contains("connection")
            || lowered.contains("server")
            || lowered.contains("disconnected")
        {
            "Open Connections to test the address, or choose Reconnect in Workspace. Your unsent request is preserved."
        } else if lowered.contains("memory")
            || lowered.contains("allocation")
            || lowered.contains("gpu")
        {
            "Open Settings → Managed runtime settings to reduce context, batch or GPU layers. Models lets you explicitly choose a smaller model."
        } else if lowered.contains("download") || lowered.contains("checksum") {
            "Open Models and choose the same file to retry. Verified files and resumable partial downloads are preserved."
        } else if lowered.contains("space") || lowered.contains("storage") {
            "Open Settings → Storage and recovery to inspect disk use, or Model files and cache to relocate managed weights."
        } else if lowered.contains("not found")
            || lowered.contains("dependencies")
            || lowered.contains("executable")
        {
            "Check the tool's prerequisite in Tools and workflows. Use Terminal jobs in Full access to install the project's documented dependency, then retry."
        } else {
            "Review the details, close this message to return to your form, and try again. Task progress provides actual check evidence and undo."
        };
        text.push_str(&format!("\n\nNext step: {next}"));
        self.status = text.clone();
        self.status_error = true;
        if let Some(dialog) = self.dialog.take()
            && !matches!(dialog, Dialog::Notice { .. })
        {
            self.recovery_dialog = Some(Box::new(dialog));
        }
        self.dialog = Some(Dialog::Notice {
            title: "Let's get this working".into(),
            body: text,
            scroll: 0,
        });
    }
    pub fn set_page(&mut self, page: Page) {
        if self.page == Page::Jobs && page != Page::Jobs {
            self.workbench.attached = false;
            self.workbench.terminal = None;
        }
        if self.job.is_some() {
            self.pending_refresh = Some(page);
        }
        let action = match page {
            Page::Files => Some("files-refresh"),
            Page::Jobs => Some("jobs-refresh"),
            Page::Context => Some("context-refresh"),
            _ => None,
        };
        if let Some(action) = action
            && self.job.is_none()
            && let Err(e) = self.workbench_action(action)
        {
            self.error(e);
        }

        self.page = page;
        self.nav_index = Page::ALL.iter().position(|p| *p == page).unwrap_or(0);
        self.nav_focus = false;
        self.search_focus = false;
        if page == Page::Task && self.job.is_none() {
            let _ = self.refresh_task();
        }
    }
    pub fn refresh_sessions(&mut self) -> Result<()> {
        if self.job.is_some() {
            self.pending_sessions = true;
            return Ok(());
        }
        let root = self.root.clone();
        let query = self.session_search.text.clone();
        let archived = self.archived;
        self.launch_io("Reading saved conversations", move || {
            let rows = Store::open(&root)?.summaries(&query, archived)?;
            Ok(JobResult::Sessions(query, archived, rows))
        })?;
        if let Some(job) = &mut self.job {
            job.replaceable = true;
        }
        Ok(())
    }
    pub fn add_message(&mut self, role: &'static str, text: impl Into<String>) {
        self.messages.push_back(Message {
            role,
            text: display_text(&text.into()),
        });
        while self.messages.len() > 200
            || self.messages.iter().map(|m| m.text.len()).sum::<usize>() > 240_000
        {
            self.messages.pop_front();
        }
    }
    pub fn apply_update(&mut self, params: &Value) {
        let update = &params["update"];
        match update["sessionUpdate"].as_str() {
            Some("agent_message_chunk") => {
                if let Some(text) = update["content"]["text"].as_str() {
                    if let Some(last) = self.messages.back_mut()
                        && last.role == "Assistant"
                        && last.text.len() < 120_000
                    {
                        last.text.push_str(&display_text(text));
                    } else {
                        self.add_message("Assistant", text);
                    }
                }
            }
            Some("tool_call" | "tool_call_update") => {
                let id = update["toolCallId"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                let index = if let Some(index) = self.tools.iter().position(|t| t.id == id) {
                    index
                } else {
                    if self.busy {
                        self.turn_tools += 1;
                    }
                    self.tools.push(Tool {
                        id: id.clone(),
                        title: "Working".into(),
                        status: "pending".into(),
                        input: Value::Null,
                        content: String::new(),
                    });
                    self.tools.len() - 1
                };
                let tool = &mut self.tools[index];
                if let Some(title) = update["title"].as_str() {
                    tool.title = display_text(title);
                }
                if let Some(status) = update["status"].as_str() {
                    tool.status = status.into();
                }
                if let Some(input) = update.get("rawInput") {
                    tool.input = input.clone();
                }
                if let Some(content) = update.get("content") {
                    tool.content = tool_content(content);
                }
                if let Some(code) = update["rawOutput"]["exit_code"].as_i64() {
                    if code != 0 {
                        tool.status = "failed".into();
                    }
                    tool.content = format!("Exit code: {code}\n{}", tool.content);
                }
                if self.tools.len() > 150 {
                    self.tools.remove(0);
                }
                self.tool_selected = self.tools.len().saturating_sub(1);
            }
            Some("usage_update") => {
                self.context_used = update["used"].as_u64();
                self.context_size = update["size"].as_u64();
            }
            _ => {}
        }
    }

    pub fn start_workspace(&mut self, resume: Option<Session>) -> Result<()> {
        ensure!(
            !self.busy && !self.connecting,
            "Stop the current task before opening another conversation"
        );
        let (name, profile) = if let Some(saved) = &resume {
            (saved.profile_name.clone(), saved.profile.clone())
        } else {
            let (name, p) = self
                .current_profile()
                .context("Choose a connection first. Open Connections to get started")?;
            (name.into(), p.clone())
        };
        ensure!(
            self.engine_ready(),
            "Install the agent engine from Home before starting a conversation"
        );
        let mut options = workspace::Options {
            root: self.root.clone(),
            requested_engine: self.requested_engine.clone(),
            preferences: self.preferences.clone(),
            profile_name: name,
            profile,
            resume,
        };
        self.launch_io("Loading conversation and project brief", move || {
            let store = Store::open(&options.root)?;
            let history = if let Some(saved) = &options.resume {
                options.preferences.choose_project(Path::new(&saved.cwd))?;
                store.recent_history(&saved.id)?
            } else {
                Vec::new()
            };
            let brief = store.brief(&options.preferences.project)?;
            Ok(JobResult::PreparedWorkspace(
                Box::new(options),
                history,
                brief,
            ))
        })
    }

    pub fn resume_workspace(&mut self, id: String) -> Result<()> {
        ensure!(
            !self.busy && !self.connecting,
            "Stop the current task before resuming a conversation"
        );
        let root = self.root.clone();
        self.launch_io("Reading saved conversation", move || {
            Ok(JobResult::Resume(Store::open(&root)?.get(&id)?))
        })
    }

    fn finish_workspace(
        &mut self,
        options: workspace::Options,
        history: Vec<Value>,
        brief: String,
    ) -> Result<()> {
        ensure!(
            !self.busy && !self.connecting,
            "Stop the current task before opening another conversation"
        );
        if let Some(old) = self.workspace.take() {
            drop(old);
        }
        self.messages.clear();
        self.tools.clear();
        self.permissions.clear();
        self.trust_session = false;
        self.trust_host = false;
        self.context_used = None;
        self.context_size = None;
        self.inference_status = None;
        self.chat_scroll = 0;
        if options.resume.is_some() {
            for event in history {
                match event["type"].as_str() {
                    Some("user") => self.add_message("You", event["text"].as_str().unwrap_or("")),
                    Some("update") => self.apply_update(&event["data"]),
                    Some("error") => {
                        self.add_message("Notice", event["text"].as_str().unwrap_or(""))
                    }
                    _ => {}
                }
            }
            self.preferences.project = options.preferences.project.clone();
            self.generation = self.generation.wrapping_add(1);
            if let Some(job) = &self.job {
                job.cancel.store(true, Ordering::Relaxed);
            }
        }
        self.brief = brief;
        self.session = options.resume.clone();
        self.connected = false;
        self.connecting = true;
        self.workspace = Some(Workspace::start(options));
        self.set_page(Page::Chat);
        self.notify("Connecting your workspace… You can keep using the interface.");
        Ok(())
    }

    pub fn send_prompt(&mut self) -> Result<()> {
        if self.composer.text.trim().is_empty() {
            return Ok(());
        }
        ensure!(
            !self.busy && !self.connecting,
            "A task is already running. Stop it or wait before sending another message"
        );
        if self.current_profile().is_none() && self.session.is_none() {
            self.connection_wizard();
            self.notify(
                "Your message is kept. Choose a model connection, then send it when ready.",
            );
            return Ok(());
        }
        if !self.connected {
            // B02: the submitted message is held separately, so anything typed
            // while connecting stays in the composer for the next message.
            self.pending_prompt = Some(self.composer.text.clone());
            self.composer.clear();
            self.start_workspace(self.session.clone())?;
            self.notify("Connecting. Your message will be sent when the model is ready; you can type the next one meanwhile.");
            return Ok(());
        }
        let text = self.composer.text.clone();
        self.composer.clear();
        self.send_text(text)
    }

    /// Put an unsent submitted message back, keeping anything typed since,
    /// so neither is lost and the message appears exactly once.
    fn restore_pending(&mut self) {
        if let Some(prompt) = self.pending_prompt.take() {
            let typed = self.composer.text.trim().to_owned();
            if typed.is_empty() {
                self.composer.replace(prompt);
            } else if typed != prompt.trim() {
                self.composer.replace(format!("{prompt}\n\n{typed}"));
            }
        }
    }

    fn send_text(&mut self, text: String) -> Result<()> {
        self.workspace
            .as_ref()
            .context("Reconnect your workspace")?
            .send(workspace::Command::Prompt {
                text: text.clone(),
                brief: self.brief.clone(),
            })?;
        self.add_message("You", text);
        self.busy = true;
        self.turn_tools = 0;
        self.cancelling = false;
        self.turn_started = Some(Instant::now());
        self.chat_scroll = 0;
        self.notify("Working. Tool activity and results will appear on the right.");
        Ok(())
    }

    pub fn stop_turn(&mut self) -> Result<()> {
        if self.connecting {
            if let Some(workspace) = self.workspace.take() {
                drop(workspace);
            }
            self.connecting = false;
            self.connected = false;
            self.restore_pending();
            self.notify("Connection cancelled. Your message is still here.");
            return Ok(());
        }
        if let Some(workspace) = &self.workspace
            && self.busy
        {
            for (id, params) in self.permissions.drain(..) {
                workspace.send(workspace::Command::Permission {
                    id,
                    params,
                    allow: false,
                })?;
            }
            workspace.send(workspace::Command::Cancel)?;
            self.cancelling = true;
            self.notify("Stopping the task…");
        }
        Ok(())
    }

    pub fn workspace_update(&mut self, update: workspace::Update) -> Result<()> {
        match update {
            workspace::Update::Ready(session) => {
                let session = *session;
                self.session = Some(session);
                self.connecting = false;
                self.connected = true;
                self.notify("Ready. Describe what you want to do.");
                self.refresh_sessions()?;
                if let Some(prompt) = self.pending_prompt.take() {
                    self.send_text(prompt)?;
                }
            }
            workspace::Update::Event(Event::Update(params)) => self.apply_update(&params),
            workspace::Update::Event(Event::Permission { id, params }) => {
                let trusted = match crate::toolbox::ApprovalScope::of(&params) {
                    crate::toolbox::ApprovalScope::Project => self.trust_session,
                    crate::toolbox::ApprovalScope::Host => self.trust_host,
                };
                if trusted && !self.cancelling {
                    self.workspace.as_ref().context("Workspace closed")?.send(
                        workspace::Command::Permission {
                            id,
                            params,
                            allow: true,
                        },
                    )?;
                } else {
                    self.permissions.push_back((id, params));
                    self.permission_selected = 0;
                    self.permission_scroll = 0;
                }
            }
            workspace::Update::Event(Event::Disconnected(reason)) => {
                self.connected = false;
                self.busy = false;
                self.connecting = false;
                self.add_message("Notice",format!("Connection closed: {reason}. Your work is saved. Choose Reconnect to continue."));
            }
            workspace::Update::Event(Event::Log(_)) => {}
            workspace::Update::Done(value) => {
                self.busy = false;
                self.cancelling = false;
                self.permissions.clear();
                let reason = value["stopReason"].as_str().unwrap_or("end_turn");
                if reason == "cancelled" {
                    self.notify("Task stopped. You can continue with another message.");
                } else if self.turn_tools == 0 {
                    self.add_message("Notice", "No tools ran for this message. Earlier results do not verify a fresh check.");
                    self.notify("Response ready. No tools ran in this turn.");
                } else {
                    self.notify(format!(
                        "Response ready. {} tool action(s) observed; review their results.",
                        self.turn_tools
                    ));
                }
                if self.job.is_none() {
                    // Read once in the worker. Starting a refresh and also reading
                    // here races the same exclusive project lock and blocks input.
                    self.refresh_task_with_notice(self.turn_tools > 0 && reason != "cancelled")?;
                } else {
                    self.pending_task_notice = Some(self.turn_tools > 0 && reason != "cancelled");
                }
                self.refresh_sessions()?;
                if self.cutoff_pending && reason != "cancelled" {
                    self.cutoff_pending = false;
                    self.add_message("Notice", "The model used its whole output allowance on thinking and gave no answer or tool call. Those tokens stay charged. Choose a recovery; nothing is resent automatically.");
                    self.cutoff_recovery()?;
                }
            }
            workspace::Update::Error(error) => {
                self.busy = false;
                self.connecting = false;
                self.permissions.clear();
                self.restore_pending();
                self.add_message("Notice", &error);
                self.error(error);
            }
            workspace::Update::Progress(progress) => self.progress_status(progress.stage),
            workspace::Update::Inference(status) => {
                let seen = self
                    .inference_status
                    .as_ref()
                    .filter(|s| s.connection_id == status.connection_id)
                    .map_or(0, |s| s.reasoning_cutoffs);
                if status.reasoning_cutoffs > seen {
                    self.cutoff_pending = true;
                }
                self.inference_status = Some(status);
                if self.cutoff_pending && !self.busy && self.dialog.is_none() {
                    self.cutoff_pending = false;
                    self.cutoff_recovery()?;
                }
            }
            workspace::Update::Closed => {
                self.connected = false;
                self.connecting = false;
                self.restore_pending();
                self.busy = false;
                self.permissions.clear();
                self.workspace = None;
            }
        }
        Ok(())
    }

    pub fn permission(&mut self, choice: usize) -> Result<()> {
        if let Some((id, params)) = self.permissions.pop_front() {
            let allow = choice > 0;
            if choice == 2 {
                // A blanket choice covers only the scope the user is looking at.
                match crate::toolbox::ApprovalScope::of(&params) {
                    crate::toolbox::ApprovalScope::Project => self.trust_session = true,
                    crate::toolbox::ApprovalScope::Host => self.trust_host = true,
                }
            }
            self.workspace
                .as_ref()
                .context("Workspace disconnected")?
                .send(workspace::Command::Permission { id, params, allow })?;
            self.notify(if allow {
                "Action allowed. Review its result in Activity."
            } else {
                "Action rejected. The assistant can adjust its plan."
            });
            self.permission_selected = 0;
            self.permission_scroll = 0;
        }
        Ok(())
    }

    pub fn launch_job<F, Fut>(&mut self, label: &str, operation: F) -> Result<()>
    where
        F: FnOnce(Cancel, mpsc::Sender<JobEvent>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<JobResult>> + Send + 'static,
    {
        if self.job.as_ref().is_some_and(|job| job.replaceable) {
            let previous = self.job.take().expect("Replaceable job");
            previous.cancel.store(true, Ordering::Relaxed);
            // Let its blocking worker observe cancellation and release the
            // project lock. Its eventual event is ignored by the scoped ID.
            // Requested saves/checks/downloads are never marked replaceable.
        }
        ensure!(
            self.job.is_none(),
            "Another background operation is still running. Wait, or pause it first"
        );
        let cancel = Arc::new(AtomicBool::new(false));
        let cancellation = cancel.clone();
        let tx = self.job_tx.clone();
        let id = uuid::Uuid::new_v4();
        let generation = self.generation;
        let task = tokio::spawn(async move {
            let (progress_tx, mut progress_rx) = mpsc::channel(16);
            let future = operation(cancellation, progress_tx);
            tokio::pin!(future);
            let result = loop {
                tokio::select! {
                    result = &mut future => break result.map_err(|e| format!("{e:#}")),
                    Some(event) = progress_rx.recv() => {
                        let _ = tx.try_send(ScopedJobEvent { id, generation, event });
                    }
                }
            };
            let _ = tx
                .send(ScopedJobEvent {
                    id,
                    generation,
                    event: JobEvent::Finished(Box::new(result)),
                })
                .await;
        });
        self.job = Some(Job {
            id,
            label: label.into(),
            progress: Progress {
                stage: "Starting; Esc requests cancellation".into(),
                ..Default::default()
            },
            cancel,
            task,
            started: Instant::now(),
            recovery: None,
            replaceable: false,
        });
        Ok(())
    }

    pub fn launch_io<F>(&mut self, label: &str, operation: F) -> Result<()>
    where
        F: FnOnce() -> Result<JobResult> + Send + 'static,
    {
        self.launch_job(label, move |cancel, _| async move {
            tokio::task::spawn_blocking(move || {
                ensure!(
                    !cancel.load(Ordering::Relaxed),
                    "Operation cancelled before starting"
                );
                operation()
            })
            .await?
        })
    }
    pub fn launch_project<F>(&mut self, label: &str, operation: F) -> Result<()>
    where
        F: FnOnce(&mut crate::project::Project, String) -> Result<JobResult> + Send + 'static,
    {
        let root = self.root.clone();
        let cwd = self.preferences.project.clone();
        let preferred = self.session.as_ref().map(|s| s.id.clone());
        self.launch_job(label, move |cancel, _| async move {
            crate::project_worker::run(root, cwd, cancel, move |p| {
                let task = crate::project_worker::task(p, preferred)?;
                operation(p, task)
            })
            .await
        })
    }
    pub fn job_event(&mut self, scoped: ScopedJobEvent) -> Result<()> {
        if !self.job.as_ref().is_some_and(|job| job.id == scoped.id) {
            return Ok(());
        }
        if scoped.generation != self.generation {
            if matches!(scoped.event, JobEvent::Finished(_)) {
                self.job = None;
                self.pending_refresh = None;
                self.set_page(self.page);
            }
            return Ok(());
        }
        match scoped.event {
            JobEvent::Progress(progress) => {
                if let Some(job) = &mut self.job {
                    job.progress = progress;
                }
            }
            JobEvent::Finished(result) => {
                let cancelled = self
                    .job
                    .as_ref()
                    .is_some_and(|job| job.cancel.load(Ordering::Relaxed));
                let submitted = self.job.take().and_then(|j| j.recovery);
                match *result {
                    Ok(JobResult::Resume(_) | JobResult::PreparedWorkspace(..)) if cancelled => {
                        self.notify("Conversation loading cancelled. Your message is kept.");
                    }
                    // B08: a cancelled check never reopens a dialog later.
                    Ok(JobResult::Tested(..) | JobResult::Inventory(..)) if cancelled => {
                        self.notify("Connection check cancelled. Nothing was saved.");
                    }
                    Ok(JobResult::Resume(session)) => self.start_workspace(Some(session))?,
                    Ok(JobResult::PreparedWorkspace(options, history, brief)) => {
                        self.finish_workspace(*options, history, brief)?;
                    }
                    Ok(JobResult::Brief(brief)) => {
                        self.brief = brief;
                        self.notify(
                            "Project brief saved. It will be included with your next message.",
                        );
                    }
                    Ok(JobResult::Dialog(dialog)) => {
                        if self.dialog.is_none() {
                            self.dialog = Some(dialog);
                        } else {
                            self.notify("Background operation completed. Your open form was preserved; reopen the result when ready.");
                        }
                    }
                    Ok(JobResult::Sessions(query, archived, rows)) => {
                        if self.session_search.text == query && self.archived == archived {
                            self.sessions = rows;
                            self.session_selected = self
                                .session_selected
                                .min(self.sessions.len().saturating_sub(1));
                        } else {
                            self.pending_sessions = true;
                        }
                    }
                    Ok(JobResult::Context(context)) => self.workbench.context = context,
                    Ok(JobResult::Practice(lesson)) => {
                        if cancelled {
                            self.notify(format!("Practice project created at {}. Your current project and message were kept.", lesson.project.display()));
                        } else {
                            self.choose_project(&lesson.project)?;
                            if self.composer.text.trim().is_empty() {
                                self.composer = Editor::new(&lesson.goal);
                            }
                            self.practice_guide();
                        }
                    }
                    Ok(JobResult::Jobs(jobs)) => {
                        self.workbench.jobs = jobs;
                        self.workbench.job_selected = self
                            .workbench
                            .job_selected
                            .min(self.workbench.jobs.len().saturating_sub(1));
                    }
                    Ok(JobResult::Saved(message)) => {
                        self.notify(message);
                        if matches!(self.page, Page::Files | Page::Context | Page::Task) {
                            self.set_page(self.page);
                        }
                    }
                    Ok(JobResult::ProjectFiles(files)) => {
                        self.workbench.files = files;
                        self.workbench.file_selected = 0;
                    }
                    Ok(JobResult::ManagedJob(record)) => {
                        self.notify(format!("Job {}: {}", record.spec.name, record.status));
                        self.workbench_action("jobs-refresh")?;
                        if record.status == "running" {
                            self.page = Page::Jobs;
                            self.nav_index = 9;
                            self.workbench.job_selected = self
                                .workbench
                                .jobs
                                .iter()
                                .position(|r| r.id == record.id)
                                .unwrap_or(0);
                            self.connect_terminal(record.id)?;
                        }
                    }
                    Ok(JobResult::Task(view)) => {
                        if view.announce_verification {
                            let notice = match &view.verification {
                                Some(v) if v.behavioral_acceptance => {
                                    "Required independent behavioral assertions passed on current files. Review their stated coverage in Task."
                                }
                                Some(v) if v.complete => {
                                    "Required commands passed on current files. Behavioral coverage remains unknown; review their contracts in Task."
                                }
                                Some(v) if v.requirements.is_empty() => {
                                    "Verification is not configured. Choose required checks in Task; tool activity alone does not establish completion."
                                }
                                Some(_) => {
                                    "Required checks are not complete on current files. Open Task to inspect failures or run verification."
                                }
                                None => {
                                    "Verification could not be read. Open Task to inspect the saved evidence."
                                }
                            };
                            self.add_message("Verification", notice);
                        }
                        self.task_view = *view;
                        self.change_selected = self
                            .change_selected
                            .min(self.task_view.changes.len().saturating_sub(1));
                    }
                    Ok(JobResult::Check(result)) => {
                        let message = if let Some(e) = &result.error {
                            e.clone()
                        } else {
                            format!(
                                "{}: {} · {:.2} seconds. Open Evidence for the actual output.",
                                result.name,
                                super::task::check_status(&result),
                                result.elapsed_ms as f64 / 1000.0
                            )
                        };
                        self.task_view.latest = Some(result);
                        self.notify(message);
                        let _ = self.refresh_task();
                    }
                    Ok(JobResult::Manager(action, state)) => {
                        if self.dialog.is_none() {
                            self.manager_action(&action, state)?;
                        } else {
                            self.notify("Background lookup completed; your current form was preserved. Reopen the lookup to continue.");
                        }
                    }
                    Ok(JobResult::Notice(title, body)) => {
                        if self.dialog.is_none() {
                            self.dialog = Some(Dialog::Notice {
                                title,
                                body,
                                scroll: 0,
                            });
                        } else {
                            self.notify(format!("{title}: {body}"));
                        }
                    }
                    Ok(JobResult::Hardware(report)) => {
                        let mut description = format!(
                            "{} RAM · {} available · {} threads\nGPU: {}\n{}\n{}\nChoose a starting context. Fit is an estimate; tool quality requires Check model.",
                            models::human_bytes(report.ram),
                            models::human_bytes(report.available_ram),
                            report.cpu_threads,
                            report.gpu,
                            report.driver,
                            report.isolation.detail
                        );
                        description.truncate(description.floor_char_boundary(1500));
                        self.dialog = Some(Dialog::Menu {
                            title: "Fit Alt to this computer".into(),
                            description,
                            items: report
                                .profiles
                                .into_iter()
                                .map(|p| {
                                    (
                                        format!("{} · {}K context", p.name, p.context / 1024),
                                        format!("{} — {}", p.model_hint, p.explanation),
                                        MenuAction::HardwareChoice(p.context),
                                    )
                                })
                                .collect(),
                            selected: 0,
                        });
                    }
                    Err(error) => {
                        if self.dialog.is_none() {
                            self.dialog = submitted;
                        }
                        if self.dialog.is_some() {
                            self.status = error;
                            self.status_error = true;
                        } else {
                            self.error(error);
                        }
                    }
                    Ok(JobResult::Inventory(name, models)) => {
                        self.server_source = Some(name);
                        self.server_models = models;
                        self.model_selected = 0;
                        self.notify("Model list refreshed. Select a model and choose Use model.");
                    }
                    Ok(JobResult::Search(models)) => {
                        self.hub_models = models;
                        self.hub_files.clear();
                        self.model_selected = 0;
                        self.notify("Select a repository to inspect its downloadable files.");
                    }
                    Ok(JobResult::Files(files)) => {
                        self.hub_files = files;
                        self.model_selected = 0;
                        self.notify("Choose a GGUF file. Size, license, and publisher claims are shown before download.");
                    }
                    Ok(JobResult::Tested(draft, models)) => {
                        if models.is_empty() {
                            self.error("The server is reachable but has no models. Load one in your model application, then test again.");
                        } else {
                            // An edited connection starts on its current model.
                            let selected = models
                                .iter()
                                .position(|m| m.id == draft.profile.model)
                                .unwrap_or(0);
                            self.dialog = Some(Dialog::PickModel {
                                draft: Box::new(draft),
                                models,
                                selected,
                                query: Editor::default(),
                            });
                        }
                    }
                    Ok(JobResult::Artifact(artifact)) => {
                        self.artifacts = models::library(&self.root)?;
                        self.model_tab = 0;
                        self.model_selected = self
                            .artifacts
                            .iter()
                            .position(|a| a.id == artifact.id)
                            .unwrap_or(0);
                        self.set_page(Page::Models);
                        self.notify("Model verified and added to your library. Choose Use model to connect it.");
                    }
                    Ok(JobResult::Installed(Component::CudaInference, path)) => {
                        // A08: never switch runtimes on the user's behalf.
                        self.manager_action("cuda-use-offer", serde_json::json!({"path": path}))?;
                    }
                    Ok(JobResult::Installed(component, path)) => {
                        match component {
                            Component::Engine => self.preferences.engine_path = Some(path),
                            Component::Inference | Component::CudaInference => {
                                self.preferences.runtime_path = Some(path)
                            }
                        }
                        self.preferences.save(&self.root)?;
                        self.notify(format!(
                            "{} is installed and ready.",
                            component.description()
                        ));
                    }
                }
            }
        }
        if self.job.is_none()
            && let Some(announce) = self.pending_task_notice.take()
        {
            self.refresh_task_with_notice(announce)?;
        }
        if self.job.is_none()
            && let Some(page) = self.pending_refresh.take()
            && page == self.page
        {
            self.set_page(page);
        }
        if self.job.is_none() && self.pending_sessions {
            self.pending_sessions = false;
            self.refresh_sessions()?;
        }
        Ok(())
    }

    pub fn connection_wizard(&mut self) {
        let labels = [
            ("Ollama", "A model already installed in Ollama."),
            ("LM Studio", "A model loaded in LM Studio's local server."),
            (
                "llama.cpp / ORA / other local server",
                "Connect to an OpenAI-compatible server.",
            ),
            (
                "Another compatible API",
                "Enter an address and optional key variable.",
            ),
            (
                "A GGUF file on this computer",
                "Import a model; Alt can run it in CPU mode.",
            ),
            (
                "A hosted API or another server preset",
                "NVIDIA NIM, OpenRouter, Groq, Together, DeepSeek, Mistral, Gemini, Cerebras, vLLM…",
            ),
        ];
        self.dialog=Some(Dialog::Menu{title:"Where will your model run?".into(),description:"Choose an application you already use, or use a model file directly. No coding is needed.".into(),items:labels.iter().enumerate().map(|(i,(label,hint))|(label.to_string(),hint.to_string(),MenuAction::Preset(i))).collect(),selected:0});
    }

    pub fn edit_connection(&mut self) -> Result<()> {
        let (name, p) = self
            .config
            .profiles
            .iter()
            .nth(self.connection_selected)
            .context("Add a connection first")?;
        ensure!(
            p.local_model.is_none(),
            "This connection uses a local file. Select another file in Models; context settings are editable in Settings"
        );
        self.dialog = Some(Dialog::Connection(ConnectionForm {
            fields: [
                Editor::new(name),
                Editor::new(&p.endpoint),
                Editor::new(p.api_key_env.clone().unwrap_or_default()),
            ],
            provider: p.provider,
            focus: 0,
            original: Some(name.clone()),
            dialect: p.effective_inference().dialect,
        }));
        Ok(())
    }

    pub fn menu_action(&mut self, action: MenuAction) -> Result<()> {
        self.dialog = None;
        match action {
            MenuAction::NewPractice => self.new_practice()?,
            MenuAction::Manager{action,state}=>self.manager_action(&action,state)?,
            MenuAction::StartJob{command,keep}=>self.dialog=Some(Dialog::Input { title: "Job time budget".into(), hint: "Seconds before stopping; 0 means unlimited. Lifetime choice still applies.".into(), editor: Editor::new("0"), action: InputAction::JobTimeout {command,keep}, multiline:false }),
            MenuAction::RequireCheck(name)=>self.toggle_required(name)?,
            MenuAction::Unpin(id)=>self.launch_project("Removing pinned requirement", move |p,_| { p.unpin(&id)?; Ok(JobResult::Saved("Requirement removed; refresh Context to review.".into())) })?,
            MenuAction::RunCheck(name)=>self.run_project_check(name)?,
            MenuAction::ConfigureCheck(spec)=>self.launch_project("Saving check", move |p,_| { p.set_check(&spec)?; Ok(JobResult::Saved("Check saved. Configure its purpose and evidence from Task.".into())) })?,
            MenuAction::CustomCheck=>self.dialog=Some(Dialog::Input{title:"Your project's check command".into(),hint:"For example: python3 check.py. This registers a command; it does not run it yet. Uses a disposable source copy with a 120-second timeout.".into(),editor:Editor::default(),action:InputAction::CheckCommand,multiline:false}),
            MenuAction::Access(crate::project::Policy::Trusted)=>self.dialog=Some(Dialog::Confirm{title:"Enable Full access?".into(),body:"Your model can request any terminal command, network access, installs and external tools with your account's permissions. Approvals remain available; Trust session approves subsequent requests. Terminal side effects are not covered by file undo. This mode is not an OS sandbox.".into(),action:ConfirmAction::Access(crate::project::Policy::Trusted),selected:0}),
            MenuAction::Access(policy)=>self.set_access(policy)?,
            MenuAction::HardwareChoice(context)=>self.apply_hardware_choice(context)?,
            MenuAction::Preset(4) => {
                self.set_page(Page::Models);
                self.open_browser(BrowserKind::Model, self.preferences.project.clone())?;
            }
            MenuAction::Preset(5) => {
                self.dialog = Some(Dialog::Menu {
                    title: "Choose a provider".into(),
                    description: "A preset fills the address, the key variable name and how thinking is sent. Your key stays in your environment; Alt saves only the variable name.".into(),
                    items: crate::presets::PRESETS
                        .iter()
                        .enumerate()
                        .map(|(i, p)| {
                            (
                                p.name.to_string(),
                                format!(
                                    "{}{}",
                                    p.note,
                                    p.key_env.map(|k| format!(" · key in {k}")).unwrap_or_default()
                                ),
                                MenuAction::ProviderPreset(i),
                            )
                        })
                        .collect(),
                    selected: 0,
                });
            }
            MenuAction::ProviderPreset(index) => {
                let preset = crate::presets::PRESETS
                    .get(index)
                    .context("That preset is no longer listed")?;
                self.dialog = Some(Dialog::Connection(ConnectionForm {
                    fields: [
                        Editor::new(preset.name),
                        Editor::new(preset.endpoint),
                        Editor::new(preset.key_env.unwrap_or("")),
                    ],
                    provider: preset.provider,
                    focus: 1,
                    original: None,
                    dialect: preset.dialect,
                }));
                if let Some(key) = preset.key_env.filter(|k| std::env::var_os(k).is_none()) {
                    self.error(format!(
                        "{key} is not set in Alt's environment. Quit, export it in your shell, then start Alt again; Alt never saves the key itself."
                    ));
                }
            }
            MenuAction::Preset(index) => {
                let (name, endpoint, provider) = match index {
                    0 => ("Ollama", "http://127.0.0.1:11434", Provider::Ollama),
                    1 => ("LM Studio", "http://127.0.0.1:1234/v1", Provider::Openai),
                    2 => ("Local server", "http://127.0.0.1:8080/v1", Provider::Openai),
                    _ => (
                        "My connection",
                        "http://127.0.0.1:8080/v1",
                        Provider::Openai,
                    ),
                };
                self.dialog = Some(Dialog::Connection(ConnectionForm {
                    fields: [Editor::new(name), Editor::new(endpoint), Editor::default()],
                    provider,
                    focus: 1,
                    original: None,
                    dialect: crate::inference::Dialect::Auto,
                }));
            }
            MenuAction::Template(index) => {
                self.composer.replace(TEMPLATES[index].2);
                self.set_page(Page::Chat);
                self.notify("Edit the message to describe your goal, then press Enter to send.");
            }
            MenuAction::Page(page) => self.set_page(page),
            MenuAction::New => {
                self.new_conversation()?;
            }
            MenuAction::Brief => self.edit_brief(),
            MenuAction::Project => {
                self.open_browser(BrowserKind::Project, self.preferences.project.clone())?
            }
            MenuAction::CurrentFolder => {
                let cwd =
                    std::env::current_dir().context("Alt cannot read the folder it was started in")?;
                self.choose_project(&cwd)?;
            }
            MenuAction::RecentProject(index) => {
                let path = self
                    .preferences
                    .recent_projects
                    .get(index)
                    .context("That recent project is no longer listed")?
                    .clone();
                self.choose_project(&path)?;
            }
            MenuAction::NewFolder => {
                self.dialog = Some(Dialog::Input {
                    title: "Create a project folder".into(),
                    hint: "Enter a new folder path. Alt creates it and makes it the project."
                        .into(),
                    editor: Editor::default(),
                    action: InputAction::NewProjectPath,
                    multiline: false,
                });
            }
            MenuAction::Connect => self.connection_wizard(),
            MenuAction::Setting(index) => {
                // Show the row in its tab so the result is visible, then run it.
                if let Some(tab) = (0..Self::SETTINGS_TABS.len())
                    .find(|tab| Self::settings_tab_indexes(*tab).contains(&index))
                {
                    self.settings_tab = tab;
                    self.settings_selected = Self::settings_tab_indexes(tab)
                        .iter()
                        .position(|i| *i == index)
                        .unwrap_or(0);
                }
                self.set_page(Page::Settings);
                self.setting_action(index)?;
            }
        }
        Ok(())
    }

    pub fn new_conversation(&mut self) -> Result<()> {
        ensure!(
            !self.busy && !self.connecting,
            "Stop the current task before opening a new conversation"
        );
        self.generation = self.generation.wrapping_add(1);
        self.pending_task_notice = None;
        if let Some(job) = &self.job {
            job.cancel.store(true, Ordering::Relaxed);
        }
        self.workspace = None;
        self.session = None;
        self.connected = false;
        self.messages.clear();
        self.tools.clear();
        self.composer.clear();
        self.trust_session = false;
        self.trust_host = false;
        self.set_page(Page::Chat);
        self.notify(
            "New conversation. Describe what you want to do, or choose a starting task from Home.",
        );
        Ok(())
    }

    pub fn edit_brief(&mut self) {
        let root = self.root.clone();
        let cwd = self.preferences.project.clone();
        if let Err(error) = self.launch_io("Reading project brief", move || {
            let brief = Store::open(&root)?.brief(&cwd)?;
            Ok(JobResult::Dialog(Dialog::Input{title:"Project brief".into(),hint:"Goals, decisions, constraints, and next steps. Included with each message; keep it short. Ctrl+S saves.".into(),editor:Editor::new(&brief),action:InputAction::Brief,multiline:true}))
        }) { self.error(error); }
    }

    pub fn open_browser(&mut self, kind: BrowserKind, path: PathBuf) -> Result<()> {
        ensure!(
            self.job.is_none(),
            "Wait for the current operation or press Esc to cancel it before browsing folders"
        );
        self.dialog = None;
        self.launch_job("Reading folder choices", move |cancel, _| async move {
            tokio::task::spawn_blocking(move || -> Result<JobResult> {
                let path = path
                    .canonicalize()
                    .context("That folder cannot be opened")?;
                let mut entries = std::fs::read_dir(&path)?
                    .take(50_001)
                    .take_while(|_| !cancel.load(Ordering::Relaxed))
                    .filter_map(|entry| entry.ok().map(|e| e.path()))
                    .filter(|entry| {
                        !entry
                            .file_name()
                            .is_some_and(|n| n.to_string_lossy().starts_with('.'))
                    })
                    .filter(|entry| {
                        entry.is_dir()
                            || !matches!(kind, BrowserKind::Project)
                                && (!matches!(kind, BrowserKind::Model)
                                    || entry.extension().is_some_and(|e| e == "gguf"))
                    })
                    .collect::<Vec<_>>();
                ensure!(!cancel.load(Ordering::Relaxed), "Folder listing cancelled");
                ensure!(
                    entries.len() <= 50_000,
                    "Folder contains too many choices; enter a narrower path directly"
                );
                entries.sort_by_key(|entry| {
                    (!entry.is_dir(), entry.file_name().map(|n| n.to_os_string()))
                });
                if let Some(parent) = path.parent() {
                    entries.insert(0, parent.to_path_buf());
                }
                Ok(JobResult::Dialog(Dialog::Browser {
                    kind,
                    path,
                    entries,
                    selected: 0,
                }))
            })
            .await?
        })
    }

    pub fn choose_project(&mut self, path: &Path) -> Result<()> {
        ensure!(
            !self.busy && !self.connecting,
            "Stop your current task before changing projects"
        );
        self.task_view = Default::default();
        self.preferences.choose_project(path)?;
        self.project_missing = false;
        self.generation = self.generation.wrapping_add(1);
        self.pending_task_notice = None;
        self.workbench = Default::default();
        if let Some(job) = &self.job {
            job.cancel.store(true, Ordering::Relaxed);
        }
        self.preferences.save(&self.root)?;
        self.brief.clear();
        self.workspace = None;
        self.session = None;
        self.connected = false;
        self.messages.clear();
        self.tools.clear();
        self.dialog = None;
        if let Some(warning) = crate::project::broad_root_warning(&self.preferences.project) {
            self.dialog = Some(Dialog::Notice {
                title: "This project folder is very broad".into(),
                body: warning.into(),
                scroll: 0,
            });
            self.error(warning);
            return Ok(());
        }
        self.notify("Project folder selected. Home → Prepare project checks helps set up how changes will be tested.");
        Ok(())
    }

    /// N02: a launch must never land on a dead workspace. A saved project
    /// folder that no longer exists opens this chooser; dismissing it opens
    /// the sandbox below so every page still works.
    pub fn open_project_chooser(&mut self) {
        let mut items: Vec<(String, String, MenuAction)> = Vec::new();
        if let Ok(cwd) = std::env::current_dir()
            && cwd.is_dir()
        {
            items.push((
                format!("Open {}", cwd.display()),
                "The folder Alt was started from".into(),
                MenuAction::CurrentFolder,
            ));
        }
        for (index, path) in self.preferences.recent_projects.iter().take(5).enumerate() {
            if path.is_dir() {
                items.push((
                    format!("Open {}", path.display()),
                    "Recent project".into(),
                    MenuAction::RecentProject(index),
                ));
            }
        }
        items.push((
            "Choose a project folder…".into(),
            "Browse this computer".into(),
            MenuAction::Project,
        ));
        items.push((
            "Create a new folder…".into(),
            "Make a folder and open it".into(),
            MenuAction::NewFolder,
        ));
        items.push((
            "Use the practice sandbox".into(),
            "A fresh example project; nothing outside it is touched".into(),
            MenuAction::NewPractice,
        ));
        self.dialog = Some(Dialog::Menu {
            title: "Open a project".into(),
            description: "The saved project folder is missing. Pick another folder now, or press Esc to open Alt's sandbox workspace.".into(),
            items,
            selected: 0,
        });
    }

    pub fn open_sandbox(&mut self) -> Result<()> {
        let sandbox = self.root.join("sandbox");
        std::fs::create_dir_all(&sandbox).context("Alt could not create its sandbox folder")?;
        let readme = sandbox.join("README.md");
        if !readme.exists() {
            crate::config::atomic_write(
                &readme,
                b"# Alt sandbox\n\nThis is a scratch workspace Alt opens when no project folder is available.\nChoose a project folder from Home at any time; this folder stays.\n",
            )?;
        }
        self.choose_project(&sandbox)?;
        self.notify(
            "Sandbox workspace opened. Choose a project folder from Home whenever you are ready.",
        );
        Ok(())
    }

    pub fn select_model(&mut self) -> Result<()> {
        match self.model_tab {
            0 => {
                let model = self
                    .artifacts
                    .get(self.model_selected)
                    .context("Import or download a model first")?
                    .clone();
                let name = format!("Local: {}", model.name);
                let profile = Profile {
                    provider: Provider::Openai,
                    endpoint: "http://127.0.0.1:8080/v1".into(),
                    model: model.name.clone(),
                    context_tokens: self.preferences.context_tokens,
                    max_turns: self.preferences.max_turns,
                    uncensored: model.uncensored_claim,
                    api_key_env: None,
                    local_model: Some(model.id),
                    inference: None,
                    runtime_key: None,
                };
                self.config.upsert(name.clone(), profile)?;
                self.config.default_profile = name;
                self.config.save(&self.root)?;
                self.notify(if self.runtime_ready(){"Local model selected. Start a conversation to load it."}else{"Local model selected. Install the local runtime with the Runtime button, then start a conversation."});
            }
            1 => {
                let model = self
                    .server_models
                    .get(self.model_selected)
                    .context("Refresh the server model list first")?
                    .id
                    .clone();
                let name = self
                    .server_source
                    .clone()
                    .context("Refresh the model list before selecting a model")?;
                let p = self
                    .config
                    .profiles
                    .get_mut(&name)
                    .context("Choose a connection first")?;
                p.model = model;
                self.config.default_profile = name;
                self.config.save(&self.root)?;
                self.notify("Model selected for new conversations. Existing conversations keep their original model.");
            }
            _ => {
                if self.hub_files.is_empty() {
                    let repo = self
                        .hub_models
                        .get(self.model_selected)
                        .context("Search Hugging Face or enter a repository first")?
                        .id
                        .clone();
                    self.launch_job("Reading model files", move |_, _| async move {
                        Ok(JobResult::Files(models::files(&repo).await?))
                    })?;
                } else {
                    let file = self
                        .hub_files
                        .get(self.model_selected)
                        .context("Choose a model file")?
                        .clone();
                    self.dialog = Some(Dialog::Confirm {
                        title: "Download this model?".into(),
                        body: format!(
                            "{}\n{}\n\nDownload: {}\nLicense: {}\nPublisher describes it as uncensored/abliterated: {}\n\nThe file will be checked against its published SHA256. Its download size is smaller than the total memory needed to run it. You can pause and resume.",
                            file.repo,
                            file.filename,
                            models::human_bytes(file.bytes),
                            file.license,
                            if file.uncensored_claim {
                                "yes (not a capability guarantee)"
                            } else {
                                "not established"
                            }
                        ),
                        action: ConfirmAction::Download(file),
                        selected: 0,
                    });
                }
            }
        }
        Ok(())
    }

    pub fn refresh_models(&mut self) -> Result<()> {
        self.model_selected = 0;
        match self.model_tab {
            0 => {
                self.artifacts = models::library(&self.root)?;
                self.notify("Local model library refreshed.");
            }
            1 => {
                let name = self.config.default_profile.clone();
                let profile = self
                    .current_profile()
                    .context("Add a server connection first")?
                    .1
                    .clone();
                ensure!(
                    profile.local_model.is_none(),
                    "This is a managed local-file connection. Choose a server connection to browse its models"
                );
                self.launch_job("Checking available models", move |_, _| async move {
                    Ok(JobResult::Inventory(
                        name,
                        models::inventory(&profile).await?,
                    ))
                })?;
            }
            _ => {
                let query = self.hub_query.clone();
                let variants = self.variants_only;
                self.launch_job("Searching Hugging Face", move |_, _| async move {
                    Ok(JobResult::Search(models::search(&query, variants).await?))
                })?;
            }
        }
        Ok(())
    }

    pub fn confirm(&mut self, action: ConfirmAction) -> Result<()> {
        self.dialog = None;
        match action {
            ConfirmAction::Manager { action, state } => self.manager_action(&action, state)?,
            ConfirmAction::ApplyFile(id) => {
                let policy = self.preferences.access_policy;
                self.launch_project("Saving file checkpoint", move |p, _| {
                    p.apply(&id, policy)?;
                    Ok(JobResult::Saved(
                        "File saved with an undo checkpoint. Refresh Files to review.".into(),
                    ))
                })?;
            }
            ConfirmAction::Access(policy) => self.set_access(policy)?,
            ConfirmAction::Undo(id) => {
                ensure!(!self.busy && !self.connecting, "Stop the task before undo");
                self.launch_project("Restoring file checkpoint", move |p, _| {
                    p.undo(&id)?;
                    Ok(JobResult::Saved(
                        "File restored. Run checks again to verify this version.".into(),
                    ))
                })?;
            }
            ConfirmAction::UndoTask(task) => {
                ensure!(!self.busy && !self.connecting, "Stop the task before undo");
                self.launch_project("Restoring task checkpoints", move |p, _| {
                    p.undo_task(&task)?;
                    Ok(JobResult::Saved(
                        "Tracked file edits restored. Run checks again.".into(),
                    ))
                })?;
            }
            ConfirmAction::Evaluate => {
                let root = self.root.clone();
                let preferences = self.preferences.clone();
                let profile = self.current_profile().context("Choose a model")?.1.clone();
                self.workspace = None;
                self.connected = false;
                self.launch_job("Checking model tool use", move |cancel, _| async move {
                    let result =
                        crate::hardware::evaluate(&root, &preferences, &profile, cancel).await?;
                    Ok(JobResult::Notice(
                        "Model capability result".into(),
                        serde_json::to_string_pretty(&result)?,
                    ))
                })?;
            }
            ConfirmAction::Download(file) => {
                let root = self.root.clone();
                self.launch_job("Downloading model", move |cancel, tx| async move {
                    let model = models::download(&root, file, cancel, move |p| {
                        let _ = tx.try_send(JobEvent::Progress(p));
                    })
                    .await?;
                    Ok(JobResult::Artifact(model))
                })?;
            }
            ConfirmAction::Install(component) => {
                let root = self.root.clone();
                self.launch_job("Installing runtime", move |cancel, tx| async move {
                    let path = runtime::install(&root, component, cancel, move |p| {
                        let _ = tx.try_send(JobEvent::Progress(p));
                    })
                    .await?;
                    Ok(JobResult::Installed(component, path))
                })?;
            }
            ConfirmAction::Archive(id, archived) => {
                let root = self.root.clone();
                self.launch_io("Updating saved conversation", move || {
                    Store::open(&root)?.archive(&id, archived)?;
                    Ok(JobResult::Saved(
                        if archived {
                            "Conversation archived. Switch to Archived to restore it."
                        } else {
                            "Conversation restored."
                        }
                        .into(),
                    ))
                })?;
                self.pending_sessions = true;
            }
            ConfirmAction::RemoveConnection(name) => {
                self.config.profiles.remove(&name);
                if self.config.default_profile == name {
                    self.config.default_profile = self
                        .config
                        .profiles
                        .keys()
                        .next()
                        .cloned()
                        .unwrap_or_default();
                }
                self.config.save(&self.root)?;
                self.connection_selected = 0;
                self.notify("Connection removed. Saved conversations were kept.");
            }
            ConfirmAction::Quit => self.quit = true,
        }
        Ok(())
    }

    pub fn install_dialog(&mut self, component: Component) {
        self.dialog = Some(Dialog::Confirm {
            title: "Install a local component".into(),
            body: format!(
                "{}\n\nDownload: {} from its pinned release.\nAlt verifies a pinned SHA256 and installs into its own data folder. No administrator access or system changes are needed.\n\nThe automatic installer currently supports Linux x86_64.",
                component.description(),
                models::human_bytes(component.size())
            ),
            action: ConfirmAction::Install(component),
            selected: 0,
        });
    }

    pub fn import_path(&mut self, path: PathBuf) -> Result<()> {
        let root = self.root.clone();
        self.dialog = None;
        self.launch_job("Checking your model file", move |cancel, tx| async move {
            Ok(JobResult::Artifact(
                models::import(&root, &path, false, cancel, move |p| {
                    let _ = tx.try_send(JobEvent::Progress(p));
                })
                .await?,
            ))
        })
    }

    pub fn submit_input(&mut self, action: InputAction, text: String) -> Result<()> {
        let form = self.dialog.take();
        let prior_job = self.job.as_ref().map(|job| job.id);
        let result = self.submit_input_inner(action, text);
        if result.is_err() && self.dialog.is_none() {
            self.dialog = form;
        } else if result.is_ok()
            && let Some(job) = &mut self.job
            && Some(job.id) != prior_job
        {
            job.recovery = form;
        }
        result
    }
    fn submit_input_inner(&mut self, action: InputAction, text: String) -> Result<()> {
        match action {
            InputAction::Manager { action, mut state } => {
                state["value"] = serde_json::json!(text);
                self.manager_action(&action, state)?;
            }
            InputAction::FileFilter => {
                self.workbench.filter = text;
                self.workbench.file_selected = 0;
            }
            InputAction::FileLine(path) => self.show_file_page(
                path,
                text.trim()
                    .parse()
                    .context("Enter a line number, starting at 1")?,
            )?,
            InputAction::FileText(path) => {
                ensure!(!text.is_empty(), "Enter text to find");
                self.launch_project("Finding text", move |p,_| {
                let bytes = p
                    .bytes(&path)?
                    .context("File was removed; refresh the list")?;
                let body = String::from_utf8(bytes).context("Only UTF-8 text can be searched")?;
                let mut hits = 0;
                let mut found = String::new();
                for (i, line) in body.lines().enumerate() {
                    if line.to_lowercase().contains(&text.to_lowercase()) {
                        hits += 1;
                        if hits <= 100 {
                            found.push_str(&format!(
                                "{}: {}\n",
                                i + 1,
                                line.chars().take(400).collect::<String>()
                            ));
                        }
                    }
                }
                Ok(JobResult::Dialog(Dialog::Notice {
                    title: format!("Find in {path}"),
                    body: format!(
                        "{hits} matching lines · first 100 shown\nClose, then G to open a matching line.\n\n{found}"
                    ),
                    scroll: 0,
                }))
                })?;
            }
            InputAction::NewFile => self.edit_project_file(text, true)?,
            InputAction::PinRequirement => {
                self.launch_project("Pinning requirement", move |p, _| {
                    p.pin(&text)?;
                    Ok(JobResult::Saved(
                        "Requirement pinned. Refresh Context to review.".into(),
                    ))
                })?
            }
            InputAction::JobCommand => {
                self.dialog=Some(Dialog::Menu{title:"How long should this job live?".into(),description:"Runs with Full access in the real project. You can stop it from Terminal jobs.".into(),items:vec![("While Alt is open".into(),"Stops when this Alt process exits".into(),MenuAction::StartJob{command:text.clone(),keep:false}),("Keep running after Alt closes".into(),"A persistent job you can reconnect to and stop later".into(),MenuAction::StartJob{command:text,keep:true})],selected:0});
            }
            InputAction::JobTimeout { command, keep } => self.start_terminal_job(
                command,
                keep,
                text.trim()
                    .parse()
                    .context("Enter whole seconds; 0 means unlimited")?,
            )?,
            InputAction::JobHealth(id) => {
                let root = self.root.clone();
                self.launch_job("Checking job health", move|cancel,_|async move {
                    let report=tokio::select!{ r=crate::jobs::health(&root,&id,&text)=>r?,_=models::cancelled(&cancel)=>anyhow::bail!("Health check cancelled") };
                    Ok(JobResult::Notice(if report["healthy"]==true {"Service is responding"} else {"Service health check failed"}.into(),serde_json::to_string_pretty(&report)?))
                })?;
            }
            InputAction::EditFile {
                path,
                original,
                task,
                new,
            } => {
                self.launch_project("Preparing file preview", move |p, _| {
                    p.note(
                        &task,
                        "plan",
                        "User edits a file through the project editor",
                        "user",
                    )?;
                    let c = p.prepare_edit(
                        &task,
                        &path,
                        None,
                        &original,
                        &text,
                        if new { "create" } else { "replace" },
                        "User edit from Project files",
                    )?;
                    Ok(JobResult::Dialog(Dialog::Confirm {
                        title: format!("Save {path}?"),
                        body: format!(
                            "An undo checkpoint will preserve the original.\n\n{}",
                            c.diff()
                        ),
                        action: ConfirmAction::ApplyFile(c.id),
                        selected: 0,
                    }))
                })?;
            }
            InputAction::CheckCommand => {
                ensure!(!text.trim().is_empty(), "Enter a check command");
                self.launch_project("Saving check command", move |p, _| {
                    p.set_check(&crate::project::CheckSpec {
                        name: "Project checks".into(),
                        argv: vec!["/bin/bash".into(), "-c".into(), text],
                        timeout_secs: 120,
                        contract: Default::default(),
                    })?;
                    Ok(JobResult::Saved(
                        "Check saved as a custom command. Choose its purpose and evidence in Task."
                            .into(),
                    ))
                })?;
            }
            InputAction::TaskNote => self.launch_project("Saving decision", move |p, task| {
                p.note(&task, "decision", &text, "user")?;
                Ok(JobResult::Saved("Decision saved in project memory.".into()))
            })?,
            InputAction::MemorySearch => {
                self.launch_project("Searching project memory", move |p, task| {
                    Ok(JobResult::Notice(
                        "Project memory and current files".into(),
                        p.memory(&task, &text, 20000)?,
                    ))
                })?
            }
            InputAction::SearchHub => {
                self.hub_query = text.trim().into();
                self.model_tab = 2;
                self.hub_files.clear();
                self.refresh_models()?;
            }
            InputAction::DirectRepo => {
                let repo = text.trim().to_string();
                self.model_tab = 2;
                self.launch_job("Reading repository", move |_, _| async move {
                    Ok(JobResult::Files(models::files(&repo).await?))
                })?;
            }
            InputAction::ImportPath => self.import_path(expand_path(&text))?,
            InputAction::ProjectPath => self.choose_project(&expand_path(&text))?,
            InputAction::NewProjectPath => {
                let path = expand_path(&text);
                std::fs::create_dir_all(&path).with_context(|| {
                    format!("That folder path cannot be created: {}", path.display())
                })?;
                self.choose_project(&path)?;
            }
            InputAction::Rename(id) => {
                let root = self.root.clone();
                self.launch_io("Naming conversation", move || {
                    Store::open(&root)?.rename(&id, &text)?;
                    Ok(JobResult::Saved("Conversation name saved.".into()))
                })?;
                self.pending_sessions = true;
            }
            InputAction::Brief => {
                let root = self.root.clone();
                let cwd = self.preferences.project.clone();
                self.launch_io("Saving project brief", move || {
                    Store::open(&root)?.save_brief(&cwd, &text)?;
                    Ok(JobResult::Brief(text))
                })?;
            }
            InputAction::EnginePath | InputAction::RuntimePath => {
                let path = expand_path(&text);
                ensure!(path.is_file(), "Choose an existing executable file");
                if matches!(action, InputAction::EnginePath) {
                    self.preferences.engine_path = Some(path);
                } else {
                    self.preferences.runtime_path = Some(path);
                }
                self.preferences.save(&self.root)?;
                self.notify("Executable path saved.");
            }
            InputAction::ManualModel(mut draft) => {
                draft.profile.model = text.trim().into();
                self.save_connection(*draft)?;
            }
        }
        Ok(())
    }

    pub fn save_connection(&mut self, draft: ConnectionDraft) -> Result<()> {
        draft.profile.validate()?;
        ensure!(
            !self.config.profiles.contains_key(&draft.name)
                || draft.original.as_ref() == Some(&draft.name),
            "A connection already has that name. Go back and choose a different name"
        );
        if let Some(original) = &draft.original
            && original != &draft.name
        {
            self.config.profiles.remove(original);
        }
        self.config.upsert(draft.name.clone(), draft.profile)?;
        self.config.default_profile = draft.name;
        self.config.save(&self.root)?;
        self.dialog = None;
        self.set_page(Page::Home);
        self.notify("Connection saved. Choose a project and start a conversation. Ctrl+P → Probe this model suggests settings for it.");
        Ok(())
    }
}

fn recover_settings<T: Default>(
    root: &Path,
    filename: &str,
    load: impl Fn(&Path) -> Result<T>,
    notices: &mut Vec<String>,
) -> Result<T> {
    match load(root) {
        Ok(settings) => Ok(settings),
        Err(error) => {
            let filename = if filename == "preferences.toml"
                && format!("{error:#}").contains("Could not load runtime.toml")
            {
                "runtime.toml"
            } else {
                filename
            };
            let backup = root.join(format!("{filename}.recovery-{}", uuid::Uuid::new_v4()));
            std::fs::rename(root.join(filename), &backup)
                .context("Could not preserve unreadable settings; no settings were reset")?;
            notices.push(format!("{error:#}\nPreserved at {}", backup.display()));
            load(root)
        }
    }
}

pub fn tool_content(content: &Value) -> String {
    let mut text = String::new();
    if let Some(items) = content.as_array() {
        for item in items {
            if let Some(t) = item["content"]["text"].as_str() {
                text.push_str(t);
                text.push('\n');
            }
            if item["type"] == "diff" {
                text.push_str(&format!("File: {}\n", item["path"].as_str().unwrap_or("")));
                for line in item["oldText"].as_str().unwrap_or("").lines() {
                    text.push_str(&format!("- {line}\n"));
                }
                for line in item["newText"].as_str().unwrap_or("").lines() {
                    text.push_str(&format!("+ {line}\n"));
                }
            }
            if text.len() > 64_000 {
                break;
            }
        }
    }
    display_text(&text).chars().take(64_000).collect()
}

pub fn tool_request(input: &Value) -> String {
    let mut description = String::new();
    if let Some(command) = input["command"].as_str() {
        description.push_str(&format!("Run this command:\n{command}\n\n"));
    }
    if let Some(path) = input["path"].as_str() {
        description.push_str(&format!("File or folder: {path}\n\n"));
    }
    if let Some(content) = input["content"].as_str() {
        description.push_str("Write the complete file shown below. If this file exists, its current contents will be replaced.\n\n");
        description.push_str(content);
        description.push_str("\n\n");
    }
    if let (Some(before), Some(after)) = (input["before"].as_str(), input["after"].as_str()) {
        description.push_str("Proposed text change (− removed, + added):\n");
        for line in before.lines() {
            description.push_str(&format!("− {line}\n"));
        }
        for line in after.lines() {
            description.push_str(&format!("+ {line}\n"));
        }
        description.push('\n');
    }
    description.push_str("Complete request details:\n");
    description.push_str(&serde_json::to_string_pretty(input).unwrap_or_default());
    display_text(&description)
}

fn expand_path(text: &str) -> PathBuf {
    let text = text.trim();
    if let Some(rest) = text.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(text)
}

struct Restore;
impl Drop for Restore {
    fn drop(&mut self) {
        let _ = execute!(
            std::io::stdout(),
            DisableMouseCapture,
            DisableBracketedPaste
        );
        ratatui::restore();
    }
}

pub async fn run(
    root: PathBuf,
    engine: PathBuf,
    profile: Option<String>,
    resume: Option<String>,
) -> Result<()> {
    ensure!(
        std::io::stdin().is_terminal() && std::io::stdout().is_terminal(),
        "Open Alt in a terminal, or use `alt run` in a script"
    );
    let mut app = App::load(root, engine, profile)?;
    // try_init enters raw mode and the alternate screen and installs the panic
    // hook; drawing goes through a buffered backend so each frame reaches the
    // terminal in one write, wrapped in a synchronized update. Terminals never
    // show half a frame, even when motion repaints the whole screen.
    let _ = ratatui::try_init()?;
    let mut terminal = ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(
        std::io::BufWriter::with_capacity(1 << 20, std::io::stdout()),
    ))?;
    let _restore = Restore;
    execute!(std::io::stdout(), EnableBracketedPaste)?;
    if app.preferences.mouse {
        execute!(std::io::stdout(), EnableMouseCapture)?;
    }
    let mut events = EventStream::new();
    // M01: adaptive frame pacing. 16 ms while something animates, 60 ms when
    // idle; any input or update redraws on the next 16 ms boundary at most.
    let frame_timer = tokio::time::sleep(Duration::ZERO);
    tokio::pin!(frame_timer);
    let mut last_draw = tokio::time::Instant::now();
    let mut hardware = tokio::spawn(runtime::hardware());
    let mut hardware_pending = true;
    let shutdown = crate::process::shutdown_signal();
    tokio::pin!(shutdown);
    if let Some(id) = resume {
        app.resume_workspace(id)?;
    }
    while !app.quit {
        tokio::select! {
            _=&mut shutdown=>{app.quit=true;},
            _=&mut frame_timer=>{
                crossterm::queue!(terminal.backend_mut(),crossterm::terminal::BeginSynchronizedUpdate)?;
                terminal.draw(|frame|view::draw(frame,&mut app))?;
                execute!(terminal.backend_mut(),crossterm::terminal::EndSynchronizedUpdate)?;
                last_draw=tokio::time::Instant::now();
                frame_timer.as_mut().reset(last_draw+app.motion.frame_period());
                continue;
            },
            result=&mut hardware,if hardware_pending=>{hardware_pending=false;if let Ok(info)=result{app.hardware=info;}},
            update=async{app.workspace.as_mut().expect("guarded workspace").updates.recv().await},if app.workspace.is_some()=>{
                let result=if let Some(update)=update{app.workspace_update(update)}else{app.workspace=None;app.connected=false;app.connecting=false;app.busy=false;Ok(())};
                if let Err(error)=result{app.error(error);}
            },
            update=async{app.workbench.terminal.as_mut().expect("guarded terminal").rx.recv().await},if app.workbench.terminal.is_some()=>{
                match update {Some(Ok(value))=>{app.workbench.screen=value["screen"].as_str().unwrap_or("").into();if let Ok(record)=serde_json::from_value::<crate::jobs::Record>(value["record"].clone())&&let Some(r)=app.workbench.jobs.iter_mut().find(|r|r.id==record.id){*r=record;}},Some(Err(error))=>{app.workbench.attached=false;app.workbench.terminal=None;app.notify(error);let _=app.workbench_action("jobs-refresh");},None=>{app.workbench.terminal=None;app.workbench.attached=false;}}
            },
            event=app.job_rx.recv()=>{if let Some(event)=event && let Err(error)=app.job_event(event){app.error(error);}},
            event=events.next()=>{
                let Some(event)=event else{break;};
                let result=match event? {
                    TerminalEvent::Key(key) if key.kind!=KeyEventKind::Release=>app.key(key),
                    TerminalEvent::Paste(text)=>{app.paste(&text);Ok(())},
                    TerminalEvent::Mouse(mouse) if app.preferences.mouse=>app.mouse(mouse.kind,mouse.column,mouse.row),
                    _=>Ok(()),
                };
                if let Err(error)=result{app.error(error);}
            }
        }
        // Something changed: show it within one fast frame.
        let soon = last_draw + super::motion::ACTIVE_FRAME;
        if frame_timer.deadline() > soon {
            frame_timer
                .as_mut()
                .reset(soon.max(tokio::time::Instant::now()));
        }
    }
    if let Some(job) = app.job.take() {
        job.cancel.store(true, Ordering::Relaxed);
        let mut handle = job.task;
        // Drain messages so a completed worker cannot block shutdown on a full queue.
        loop {
            tokio::select! { _ = &mut handle => break, _ = app.job_rx.recv() => {} }
        }
    }
    if let Some(mut workspace) = app.workspace.take() {
        workspace.stop().await;
    }
    app.workbench.terminal = None;
    crate::jobs::stop_owned(&app.root).await?;
    hardware.abort();
    Ok(())
}

#[cfg(test)]
mod recovery_tests {
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn pinning_a_requirement_replaces_a_locked_context_refresh() {
        use super::*;
        let data = tempfile::tempdir().unwrap();
        let cwd = tempfile::tempdir().unwrap();
        let mut app = App::load(data.path().into(), "goose".into(), None).unwrap();
        app.preferences.project = cwd.path().into();
        app.composer = Editor::new("Keep my draft");
        let locked = crate::project::Project::open(data.path(), cwd.path()).unwrap();
        app.set_page(Page::Context);
        let refresh_cancel = app.job.as_ref().unwrap().cancel.clone();
        app.workbench_action("pin-requirement").unwrap();
        app.submit_input(
            InputAction::PinRequirement,
            "Preserve the public API".into(),
        )
        .expect("Pinning a requirement must supersede a disposable context refresh");
        assert!(refresh_cancel.load(Ordering::Relaxed));
        drop(locked);
        while app.job.is_some() {
            let event = tokio::time::timeout(Duration::from_secs(5), app.job_rx.recv())
                .await
                .unwrap()
                .unwrap();
            app.job_event(event).unwrap();
        }
        assert!(app.workbench.context.contains("Preserve the public API"));
        assert_eq!(app.composer.text, "Keep my draft");
        let reopened = crate::project::Project::open(data.path(), cwd.path()).unwrap();
        assert!(
            serde_json::to_string(&reopened.pinned().unwrap())
                .unwrap()
                .contains("Preserve the public API")
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn task_check_choices_replace_a_locked_status_refresh() {
        use super::*;
        let data = tempfile::tempdir().unwrap();
        let cwd = tempfile::tempdir().unwrap();
        let mut app = App::load(data.path().into(), "goose".into(), None).unwrap();
        app.preferences.project = cwd.path().into();
        app.composer = Editor::new("Keep this unsent request");
        for action in ["task-configure", "task-check"] {
            app.dialog = None;
            let locked = crate::project::Project::open(data.path(), cwd.path()).unwrap();
            app.set_page(Page::Task);
            let refresh_cancel = app.job.as_ref().unwrap().cancel.clone();
            app.task_action(action)
                .expect("A requested check choice should supersede a disposable status refresh");
            assert!(refresh_cancel.load(Ordering::Relaxed));
            drop(locked);
            while app.job.is_some() {
                let event = tokio::time::timeout(Duration::from_secs(5), app.job_rx.recv())
                    .await
                    .unwrap()
                    .unwrap();
                app.job_event(event).unwrap();
            }
            assert!(
                matches!(&app.dialog, Some(Dialog::Menu { title, .. }) if title == "Choose what should prove the change works")
            );
            assert_eq!(app.composer.text, "Keep this unsent request");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn new_file_can_replace_a_locked_listing_and_failed_forms_keep_their_text() {
        use super::*;
        let data = tempfile::tempdir().unwrap();
        let cwd = tempfile::tempdir().unwrap();
        let mut app = App::load(data.path().into(), "goose".into(), None).unwrap();
        app.preferences.project = cwd.path().into();
        app.composer = Editor::new("Keep my unsent request");
        for already_exists in [false, true] {
            if already_exists {
                std::fs::write(cwd.path().join("notes.txt"), "Original contents").unwrap();
            }
            app.dialog = None;
            let locked = crate::project::Project::open(data.path(), cwd.path()).unwrap();
            app.set_page(Page::Files);
            let listing_cancel = app.job.as_ref().unwrap().cancel.clone();
            app.workbench_action("file-new").unwrap();
            if let Some(Dialog::Input { editor, .. }) = &mut app.dialog {
                editor.replace("notes.txt");
            }
            app.submit_input(InputAction::NewFile, "notes.txt".into())
                .expect("Opening the editor should supersede a disposable file listing");
            assert!(listing_cancel.load(Ordering::Relaxed));
            drop(locked);
            while app.job.is_some() {
                let event = tokio::time::timeout(Duration::from_secs(5), app.job_rx.recv())
                    .await
                    .unwrap()
                    .unwrap();
                app.job_event(event).unwrap();
            }
            if already_exists {
                assert!(
                    matches!(&app.dialog, Some(Dialog::Input { editor, action: InputAction::NewFile, .. }) if editor.text == "notes.txt")
                );
                assert!(app.status.contains("already exists"));
                assert_eq!(
                    std::fs::read_to_string(cwd.path().join("notes.txt")).unwrap(),
                    "Original contents"
                );
            } else {
                assert!(
                    matches!(&app.dialog, Some(Dialog::Input { title, action: InputAction::EditFile { new: true, .. }, .. }) if title == "Edit notes.txt")
                );
                assert!(!cwd.path().join("notes.txt").exists());
            }
            assert_eq!(app.composer.text, "Keep my unsent request");
        }
    }

    #[tokio::test]
    async fn explicit_operations_never_replace_a_requested_save() {
        use super::*;
        let data = tempfile::tempdir().unwrap();
        let mut app = App::load(data.path().into(), "goose".into(), None).unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        app.launch_job("Saving user work", move |_, _| async move {
            rx.await?;
            Ok(JobResult::Saved("User work saved".into()))
        })
        .unwrap();
        let original = app.job.as_ref().unwrap().id;
        assert!(
            app.launch_io("New operation", || Ok(JobResult::Saved("wrong".into())))
                .is_err()
        );
        assert_eq!(app.job.as_ref().unwrap().id, original);
        assert!(!app.job.as_ref().unwrap().cancel.load(Ordering::Relaxed));
        tx.send(()).unwrap();
        let event = app.job_rx.recv().await.unwrap();
        app.job_event(event).unwrap();
        assert!(app.status.contains("User work saved"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn conversation_database_contention_keeps_drafts_responsive_and_saves_after_release() {
        use super::*;
        let data = tempfile::tempdir().unwrap();
        let mut app = App::load(data.path().into(), "goose".into(), None).unwrap();
        let writer = rusqlite::Connection::open(data.path().join("sessions.db")).unwrap();
        writer.execute_batch("BEGIN IMMEDIATE;").unwrap();
        app.composer = Editor::new("Keep this unsent message 🦀");
        let started = Instant::now();
        app.submit_input(InputAction::Brief, "Saved after contention".into())
            .unwrap();
        assert!(started.elapsed() < Duration::from_millis(250));
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(app.job.is_some());
        assert_ne!(app.brief, "Saved after contention");
        writer.execute_batch("ROLLBACK;").unwrap();
        let result = tokio::time::timeout(Duration::from_secs(5), app.job_rx.recv())
            .await
            .unwrap()
            .unwrap();
        app.job_event(result).unwrap();
        assert_eq!(app.brief, "Saved after contention");
        assert_eq!(app.composer.text, "Keep this unsent message 🦀");
        assert_eq!(
            Store::open(data.path())
                .unwrap()
                .brief(&app.preferences.project)
                .unwrap(),
            app.brief
        );
    }

    #[tokio::test]
    async fn cancelled_conversation_loading_never_opens_an_agent() {
        use super::*;
        let data = tempfile::tempdir().unwrap();
        let mut app = App::load(data.path().into(), "/missing-engine".into(), None).unwrap();
        let session = Session {
            id: "cancelled-session".into(),
            engine_id: "unused".into(),
            cwd: data.path().to_string_lossy().into_owned(),
            profile_name: "fixture".into(),
            profile: Profile {
                provider: Provider::Openai,
                endpoint: "http://127.0.0.1:1/v1".into(),
                model: "no-weights-fixture".into(),
                context_tokens: 2048,
                max_turns: 2,
                uncensored: false,
                api_key_env: None,
                local_model: None,
                inference: None,
                runtime_key: None,
            },
        };
        app.launch_job("Loading", move |_, _| async move {
            Ok(JobResult::Resume(session))
        })
        .unwrap();
        app.job
            .as_ref()
            .unwrap()
            .cancel
            .store(true, Ordering::Relaxed);
        let result = app.job_rx.recv().await.unwrap();
        app.job_event(result).unwrap();
        assert!(app.workspace.is_none());
        assert!(!app.connecting);
        assert!(app.status.contains("cancelled"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn turn_end_refresh_does_not_race_itself_or_replace_an_open_form() {
        use super::*;
        let data = tempfile::tempdir().unwrap();
        let cwd = tempfile::tempdir().unwrap();
        for n in 0..32 {
            std::fs::write(
                cwd.path().join(format!("source-{n}.txt")),
                "x".repeat(32 * 1024),
            )
            .unwrap();
        }
        let p = crate::project::Project::open(data.path(), cwd.path()).unwrap();
        p.start_task("t", "Verify task").unwrap();
        p.set_check(&crate::project::CheckSpec {
            name: "check".into(),
            argv: vec!["true".into()],
            timeout_secs: 10,
            contract: Default::default(),
        })
        .unwrap();
        p.set_requirement(&crate::project_services::Requirement {
            name: "required".into(),
            description: "Run check".into(),
            check_name: "check".into(),
        })
        .unwrap();
        drop(p);
        let mut app = App::load(data.path().into(), "goose".into(), None).unwrap();
        app.preferences.project = cwd.path().into();
        app.turn_tools = 1;
        for _ in 0..8 {
            app.workspace_update(workspace::Update::Done(
                serde_json::json!({"stopReason":"end_turn"}),
            ))
            .unwrap();
            app.dialog = Some(Dialog::Input {
                title: "Project brief".into(),
                hint: String::new(),
                editor: Editor::new("Keep my draft"),
                action: InputAction::Brief,
                multiline: true,
            });
            let event = tokio::time::timeout(Duration::from_secs(5), app.job_rx.recv())
                .await
                .unwrap()
                .unwrap();
            app.job_event(event)
                .expect("One turn-end refresh must not compete with itself for the project lock");
            assert!(
                matches!(&app.dialog, Some(Dialog::Input {editor,..}) if editor.text=="Keep my draft")
            );
            assert!(app.task_view.verification.is_some());
        }
    }
    #[tokio::test]
    async fn old_project_results_and_failed_background_forms_do_not_replace_newer_work() {
        use super::*;
        let data = tempfile::tempdir().unwrap();
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let mut app = App::load(data.path().into(), "goose".into(), None).unwrap();
        app.preferences.project = a.path().into();
        let (tx, rx) = tokio::sync::oneshot::channel();
        app.launch_job("Old project", move |_, _| async move {
            rx.await?;
            Ok(JobResult::ProjectFiles(vec!["old.txt".into()]))
        })
        .unwrap();
        app.choose_project(b.path()).unwrap();
        tx.send(()).unwrap();
        let event = app.job_rx.recv().await.unwrap();
        app.job_event(event).unwrap();
        assert!(app.workbench.files.is_empty());
        app.dialog = Some(Dialog::Input {
            title: "Find".into(),
            hint: String::new(),
            editor: Editor::new("preserve my text"),
            action: InputAction::FileText("missing".into()),
            multiline: false,
        });
        app.submit_input(
            InputAction::FileText("missing".into()),
            "preserve my text".into(),
        )
        .unwrap();
        let event = app.job_rx.recv().await.unwrap();
        app.job_event(event).unwrap();
        assert!(
            matches!(&app.dialog,Some(Dialog::Input{editor,..}) if editor.text=="preserve my text")
        );
        assert!(app.status_error);
    }
    use super::*;
    #[test]
    fn damaged_runtime_is_preserved_without_resetting_project_preferences() {
        let root = tempfile::tempdir().unwrap();
        let preferences = Preferences {
            project: PathBuf::from("/tmp/cedar-project"),
            ..Preferences::default()
        };
        preferences.save(root.path()).unwrap();
        // Exercise migration from the pre-0.5 two-file layout.
        let mut legacy = toml::Value::try_from(&preferences).unwrap();
        legacy.as_table_mut().unwrap().remove("runtime");
        crate::config::atomic_write(
            &root.path().join("preferences.toml"),
            toml::to_string(&legacy).unwrap().as_bytes(),
        )
        .unwrap();
        let original = std::fs::read(root.path().join("preferences.toml")).unwrap();
        std::fs::write(root.path().join("runtime.toml"), "gpu_layers = [broken").unwrap();
        let mut notices = Vec::new();
        let loaded = recover_settings(
            root.path(),
            "preferences.toml",
            Preferences::load,
            &mut notices,
        )
        .unwrap();
        assert_eq!(loaded.project, preferences.project);
        assert_eq!(
            std::fs::read(root.path().join("preferences.toml")).unwrap(),
            original
        );
        assert_eq!(notices.len(), 1);
        assert!(std::fs::read_dir(root.path()).unwrap().any(|p| {
            p.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("runtime.toml.recovery-")
        }));
        assert!(Preferences::load(root.path()).is_ok());
    }

    #[test]
    fn a_dead_saved_project_opens_a_chooser_and_esc_lands_in_the_sandbox() {
        let data = tempfile::tempdir().unwrap();
        let gone = data.path().join("deleted-project");
        std::fs::write(
            data.path().join("preferences.toml"),
            format!("project = {:?}\n", gone.display().to_string()),
        )
        .unwrap();
        let mut app = App::load(data.path().into(), "goose".into(), None).unwrap();
        assert!(
            app.project_missing,
            "a dead project must be detected at load"
        );
        assert!(
            matches!(&app.dialog, Some(Dialog::Menu { title, .. }) if title == "Open a project"),
            "launching on a dead project must open the chooser"
        );
        app.key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Esc,
        ))
        .unwrap();
        assert!(
            !app.project_missing,
            "dismissing the chooser must leave a working workspace"
        );
        assert!(app.dialog.is_none());
        let sandbox = data.path().join("sandbox");
        assert_eq!(app.preferences.project, sandbox);
        assert!(sandbox.join("README.md").exists());
        let stored = crate::config::Preferences::load(data.path()).unwrap();
        assert_eq!(stored.project, sandbox);
    }

    #[test]
    fn settings_tabs_cover_every_row_exactly_once() {
        let data = tempfile::tempdir().unwrap();
        let app = App::load(data.path().into(), "goose".into(), None).unwrap();
        let rows = app.settings_all_items(8192, 12);
        let mut seen = std::collections::BTreeSet::new();
        for tab in 0..App::SETTINGS_TABS.len() {
            for index in App::settings_tab_indexes(tab) {
                assert!(seen.insert(*index), "setting {index} appears twice");
            }
        }
        assert_eq!(
            seen,
            (0..rows.len()).collect::<std::collections::BTreeSet<_>>(),
            "every settings row must sit in exactly one tab"
        );
    }

    #[test]
    fn palette_filter_matches_words_and_runs_pages() {
        let data = tempfile::tempdir().unwrap();
        let mut app = App::load(data.path().into(), "goose".into(), None).unwrap();
        app.dialog = Some(Dialog::Palette {
            query: Editor::default(),
            selected: 0,
        });
        assert!(app.palette_filtered().len() >= 15);
        let mut query = Editor::default();
        query.insert("theme", false);
        app.dialog = Some(Dialog::Palette { query, selected: 0 });
        let filtered = app.palette_filtered();
        assert!(
            filtered
                .iter()
                .any(|(label, _, _)| label.contains("Appearance")),
            "the theme picker must be findable"
        );
        assert!(
            !filtered
                .iter()
                .any(|(label, _, _)| label.contains("Terminal jobs")),
            "unrelated pages must be filtered out"
        );
    }

    #[test]
    fn every_settings_row_is_in_the_palette() {
        let data = tempfile::tempdir().unwrap();
        let app = App::load(data.path().into(), "goose".into(), None).unwrap();
        let rows = app.settings_all_items(8192, 12).len();
        let reachable: std::collections::BTreeSet<usize> = app
            .palette_items()
            .into_iter()
            .filter_map(|(_, _, action)| match action {
                MenuAction::Setting(index) => Some(index),
                _ => None,
            })
            .collect();
        assert_eq!(reachable, (0..rows).collect());
    }

    #[test]
    fn session_trust_never_covers_terminal_requests() {
        let data = tempfile::tempdir().unwrap();
        let mut app = App::load(data.path().into(), "goose".into(), None).unwrap();
        let host =
            serde_json::json!({"toolCall":{"title":"Terminal"},"_meta":{"alt":{"scope":"host"}}});
        let project =
            serde_json::json!({"toolCall":{"title":"Edit"},"_meta":{"alt":{"scope":"project"}}});
        app.trust_session = true;
        app.workspace_update(workspace::Update::Event(Event::Permission {
            id: serde_json::json!(1),
            params: host.clone(),
        }))
        .unwrap();
        assert_eq!(app.permissions.len(), 1, "terminal must still ask");
        // Choosing the blanket option on a project request trusts project scope only.
        app.trust_session = false;
        app.permissions.clear();
        app.permissions.push_back((serde_json::json!(2), project));
        let _ = app.permission(2);
        assert!(app.trust_session && !app.trust_host);
        app.permissions.push_back((serde_json::json!(3), host));
        let _ = app.permission(2);
        assert!(app.trust_host);
    }

    #[tokio::test]
    async fn a_reasoning_cutoff_offers_recovery_without_resending() {
        let data = tempfile::tempdir().unwrap();
        let mut app = App::load(data.path().into(), "goose".into(), None).unwrap();
        let mut status = crate::inference::Status {
            connection_id: "c1".into(),
            stage: crate::inference::Stage::Ready,
            requests_issued: 1,
            remaining_requests: None,
            remaining_generated_tokens: None,
            input_tokens: None,
            input_allowance: 6144,
            output_allowance: 2048,
            action_headroom: 256,
            reasoning_allowance: None,
            reasoning_cutoffs: 0,
            ollama_context: None,
            provider_retries: 0,
            provider_faults: 0,
        };
        app.workspace_update(workspace::Update::Inference(status.clone()))
            .unwrap();
        assert!(app.dialog.is_none());
        status.reasoning_cutoffs = 1;
        status.stage = crate::inference::Stage::ReasoningCutoff;
        app.busy = true;
        app.workspace_update(workspace::Update::Inference(status.clone()))
            .unwrap();
        assert!(app.dialog.is_none(), "wait for the turn to end");
        app.workspace_update(workspace::Update::Done(
            serde_json::json!({"stopReason":"end_turn"}),
        ))
        .unwrap();
        match &app.dialog {
            Some(Dialog::Menu { title, items, .. }) => {
                assert!(title.contains("thinking"));
                assert!(
                    items
                        .iter()
                        .any(|(label, _, _)| label == "Resend with thinking off")
                );
            }
            _ => panic!("cutoff recovery menu expected"),
        }
        // The same count on the same connection does not reopen it.
        app.dialog = None;
        app.workspace_update(workspace::Update::Inference(status))
            .unwrap();
        assert!(app.dialog.is_none());
    }

    #[test]
    fn hosted_presets_fill_the_form_and_the_picker_filters_long_catalogs() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let data = tempfile::tempdir().unwrap();
        let mut app = App::load(data.path().into(), "goose".into(), None).unwrap();
        let nvidia = crate::presets::PRESETS
            .iter()
            .position(|p| p.id == "nvidia")
            .unwrap();
        app.menu_action(MenuAction::ProviderPreset(nvidia)).unwrap();
        let form = match app.recovery_dialog.as_deref().or(app.dialog.as_ref()) {
            Some(Dialog::Connection(form)) => form.clone(),
            other => panic!("connection form expected, got {other:?}"),
        };
        assert_eq!(form.fields[1].text, "https://integrate.api.nvidia.com/v1");
        assert_eq!(form.fields[2].text, "NVIDIA_API_KEY");
        assert_eq!(form.dialect, crate::inference::Dialect::ChatTemplate);
        let draft = app.draft(&form).unwrap();
        let models = ["z-ai/glm-5.3", "moonshotai/kimi-k3", "z-ai/glm-4.6"]
            .into_iter()
            .map(|id| models::ServerModel {
                id: id.into(),
                bytes: None,
                parameters: None,
                quant: None,
                family: None,
                modified: None,
            })
            .collect();
        app.dialog = Some(Dialog::PickModel {
            draft: Box::new(draft),
            models,
            selected: 0,
            query: Editor::default(),
        });
        for c in "kimi".chars() {
            app.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
                .unwrap();
        }
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .unwrap();
        let (_, saved) = app.current_profile().unwrap();
        assert_eq!(saved.model, "moonshotai/kimi-k3");
        assert_eq!(saved.api_key_env.as_deref(), Some("NVIDIA_API_KEY"));
        assert_eq!(
            saved.effective_inference().dialect,
            crate::inference::Dialect::ChatTemplate
        );
        let text = std::fs::read_to_string(data.path().join("config.toml")).unwrap();
        assert!(text.contains("NVIDIA_API_KEY") && text.contains("chat-template"));
    }

    #[test]
    fn editing_a_connection_keeps_its_custom_settings() {
        // Audit A03: a customised 4K profile must survive a name/endpoint edit.
        let data = tempfile::tempdir().unwrap();
        let mut settings = crate::inference::Settings::legacy(4096);
        settings.output_tokens = 1024;
        settings.max_requests = Some(9);
        settings.total_generated_tokens = Some(5000);
        settings.temperature = Some(0.3);
        settings.thinking = crate::inference::Thinking::Off;
        let profile = Profile {
            provider: Provider::Openai,
            endpoint: "http://127.0.0.1:8080/v1".into(),
            model: "mimo-9b".into(),
            context_tokens: 4096,
            max_turns: 7,
            uncensored: true,
            api_key_env: None,
            local_model: None,
            inference: Some(settings.clone()),
            runtime_key: None,
        };
        Config::create(data.path(), profile).unwrap();
        let mut app = App::load(data.path().into(), "goose".into(), None).unwrap();
        app.edit_connection().unwrap();
        let mut form = match app.dialog.clone() {
            Some(Dialog::Connection(form)) => form,
            _ => panic!("connection form expected"),
        };
        form.fields[0] = Editor::new("renamed");
        form.fields[1] = Editor::new("http://127.0.0.1:8081/v1");
        let draft = app.draft(&form).unwrap();
        assert_eq!(draft.profile.model, "mimo-9b");
        app.save_connection(draft).unwrap();
        let saved = &app.config.profiles["renamed"];
        assert!(!app.config.profiles.contains_key("local"));
        assert_eq!(saved.endpoint, "http://127.0.0.1:8081/v1");
        assert_eq!(saved.context_tokens, 4096);
        assert_eq!(saved.max_turns, 7);
        assert!(saved.uncensored);
        assert_eq!(
            serde_json::to_value(saved.effective_inference()).unwrap(),
            serde_json::to_value(settings).unwrap()
        );
    }

    #[test]
    fn text_typed_while_connecting_survives_failure_and_cancel_exactly_once() {
        let data = tempfile::tempdir().unwrap();
        let mut app = App::load(data.path().into(), "goose".into(), None).unwrap();
        // Failure: the submitted message comes back first, the new draft after it.
        app.pending_prompt = Some("Fix the parser".into());
        app.composer.replace("Also update the docs");
        app.workspace_update(workspace::Update::Error("Server unavailable".into()))
            .unwrap();
        assert_eq!(app.composer.text, "Fix the parser\n\nAlso update the docs");
        assert!(app.pending_prompt.is_none());
        // Cancelling a connection restores it without duplicating identical text.
        app.composer.replace("Fix the parser");
        app.pending_prompt = Some("Fix the parser".into());
        app.connecting = true;
        app.stop_turn().unwrap();
        assert_eq!(app.composer.text, "Fix the parser");
        // A closed connection also gives the message back.
        app.composer.clear();
        app.pending_prompt = Some("Run the tests".into());
        app.workspace_update(workspace::Update::Closed).unwrap();
        assert_eq!(app.composer.text, "Run the tests");
    }

    #[test]
    fn manual_model_entry_has_a_key_terminals_can_send() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let data = tempfile::tempdir().unwrap();
        let mut app = App::load(data.path().into(), "goose".into(), None).unwrap();
        app.menu_action(MenuAction::Preset(1)).unwrap();
        assert!(matches!(app.dialog, Some(Dialog::Connection(_))));
        app.key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::ALT))
            .unwrap();
        match &app.dialog {
            Some(Dialog::Input { title, action, .. }) => {
                assert_eq!(title, "Enter a model identifier");
                assert!(matches!(action, InputAction::ManualModel(_)));
            }
            _ => panic!("manual model entry expected"),
        }
        let mut editor = Editor::default();
        editor.insert("my-exact-model:7b", false);
        if let Some(Dialog::Input { editor: e, .. }) = &mut app.dialog {
            *e = editor;
        }
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .unwrap();
        assert_eq!(app.current_profile().unwrap().1.model, "my-exact-model:7b");
    }

    #[tokio::test]
    async fn a_cancelled_connection_check_returns_quickly_and_opens_nothing() {
        // Audit A19: a stalled server, then Esc.
        let stalled = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = stalled.local_addr().unwrap();
        let hold = tokio::spawn(async move {
            let (_socket, _) = stalled.accept().await.unwrap();
            std::future::pending::<()>().await;
        });
        let data = tempfile::tempdir().unwrap();
        let mut app = App::load(data.path().into(), "goose".into(), None).unwrap();
        app.menu_action(MenuAction::Preset(1)).unwrap();
        if let Some(Dialog::Connection(form)) = &mut app.dialog {
            form.fields[1] = Editor::new(format!("http://{address}/v1"));
        }
        app.test_form().unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        let started = Instant::now();
        app.job
            .as_ref()
            .unwrap()
            .cancel
            .store(true, Ordering::Relaxed);
        let event = tokio::time::timeout(Duration::from_secs(1), app.job_rx.recv())
            .await
            .expect("cancellation must return control within one second")
            .unwrap();
        app.job_event(event).unwrap();
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(app.job.is_none());
        assert!(
            matches!(app.dialog, Some(Dialog::Connection(_))),
            "the form stays; no model dialog opens"
        );
        hold.abort();
    }

    #[test]
    fn an_oversized_paste_is_refused_with_a_reason() {
        let data = tempfile::tempdir().unwrap();
        let mut app = App::load(data.path().into(), "goose".into(), None).unwrap();
        app.page = Page::Chat;
        app.composer.replace("keep this");
        app.paste(&"x".repeat(70 * 1024));
        assert_eq!(app.composer.text, "keep this", "nothing inserted");
        assert!(app.status_error);
        assert!(
            app.status.contains("70 KiB") || app.dialog.is_some(),
            "{}",
            app.status
        );
        app.dialog = None;
        app.paste(" and this");
        assert_eq!(app.composer.text, "keep this and this");
    }
}
