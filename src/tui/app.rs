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

#[derive(Debug, Clone)]
pub enum Hit {
    Nav(Page),
    Home(usize),
    Row(usize),
    Button(&'static str),
    ModelTab(usize),
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
    ManualModel(ConnectionDraft),
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
    Manager { action: String, state: Value },
    StartJob { command: String, keep: bool },
    RequireCheck(String),
    Unpin(String),
    RunCheck(String),
    ConfigureCheck(crate::project::CheckSpec),
    CustomCheck,
    Access(crate::project::Policy),
    HardwareChoice(u32),
    Preset(usize),
    Template(usize),
    Page(Page),
    New,
    Brief,
    Project,
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
        draft: ConnectionDraft,
        models: Vec<String>,
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
    Inventory(String, Vec<String>),
    Search(Vec<HubModel>),
    Files(Vec<ModelFile>),
    Tested(ConnectionDraft, Vec<String>),
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
    pub home_selected: usize,
    pub connection_selected: usize,
    pub session_selected: usize,
    pub settings_selected: usize,
    pub sessions: Vec<SessionSummary>,
    pub session_search: Editor,
    pub search_focus: bool,
    pub archived: bool,
    pub model_tab: usize,
    pub model_selected: usize,
    pub artifacts: Vec<Artifact>,
    pub server_models: Vec<String>,
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
    pub context_used: Option<u64>,
    pub context_size: Option<u64>,
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
            home_selected: 0,
            connection_selected: 0,
            session_selected: 0,
            settings_selected: 0,
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
            context_used: None,
            context_size: None,
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
        Ok(app)
    }

    pub fn current_profile(&self) -> Option<(&str, &Profile)> {
        self.config.profile(None).ok()
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
        })
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
        self.context_used = None;
        self.context_size = None;
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
            self.pending_prompt = Some(self.composer.text.clone());
            self.start_workspace(self.session.clone())?;
            return Ok(());
        }
        let text = self.composer.text.clone();
        self.workspace
            .as_ref()
            .context("Reconnect your workspace")?
            .send(workspace::Command::Prompt {
                text: text.clone(),
                brief: self.brief.clone(),
            })?;
        self.composer.clear();
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
            self.pending_prompt = None;
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
                self.session = Some(session);
                self.connecting = false;
                self.connected = true;
                self.notify("Ready. Describe what you want to do.");
                self.refresh_sessions()?;
                if let Some(prompt) = self.pending_prompt.take() {
                    self.composer.replace(prompt);
                    self.send_prompt()?;
                }
            }
            workspace::Update::Event(Event::Update(params)) => self.apply_update(&params),
            workspace::Update::Event(Event::Permission { id, params }) => {
                if self.trust_session && !self.cancelling {
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
            }
            workspace::Update::Error(error) => {
                self.busy = false;
                self.connecting = false;
                self.permissions.clear();
                if let Some(prompt) = self.pending_prompt.take() {
                    self.composer.replace(prompt);
                }
                self.add_message("Notice", &error);
                self.error(error);
            }
            workspace::Update::Progress(progress) => self.notify(progress.stage),
            workspace::Update::Closed => {
                self.connected = false;
                self.connecting = false;
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
                self.trust_session = true;
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
                            self.dialog = Some(Dialog::PickModel {
                                draft,
                                models,
                                selected: 0,
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
                    Ok(JobResult::Installed(component, path)) => {
                        match component {
                            Component::Engine => self.preferences.engine_path = Some(path),
                            Component::Inference => self.preferences.runtime_path = Some(path),
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
        }));
        Ok(())
    }

    pub fn menu_action(&mut self, action: MenuAction) -> Result<()> {
        self.dialog = None;
        match action {
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
        self.notify("Project folder selected. New conversations will work in this folder.");
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
                "{}\n\nDownload: {} from the official GitHub release.\nAlt verifies a pinned SHA256 and installs into its own data folder. No administrator access or system changes are needed.\n\nThe automatic installer currently supports Linux x86_64.",
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
        let had_job = self.job.is_some();
        let result = self.submit_input_inner(action, text);
        if result.is_err() && self.dialog.is_none() {
            self.dialog = form;
        } else if result.is_ok()
            && !had_job
            && let Some(job) = &mut self.job
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
                self.save_connection(draft)?;
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
        self.notify("Connection saved. Choose a project and start a conversation.");
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
    let mut terminal = ratatui::try_init()?;
    let _restore = Restore;
    execute!(std::io::stdout(), EnableBracketedPaste)?;
    if app.preferences.mouse {
        execute!(std::io::stdout(), EnableMouseCapture)?;
    }
    let mut events = EventStream::new();
    let mut tick = tokio::time::interval(Duration::from_millis(60));
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
            _=tick.tick()=>{terminal.draw(|frame|view::draw(frame,&mut app))?;},
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
}
