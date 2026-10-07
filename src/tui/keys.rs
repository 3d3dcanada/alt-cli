use super::{app::*, input::Editor};
use crate::{config::Profile, models, runtime::Component};
use anyhow::{Context, Result, ensure};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use std::sync::atomic::Ordering;

impl App {
    pub fn key(&mut self, key: KeyEvent) -> Result<()> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        if key.code == KeyCode::Esc
            && self.dialog.is_none()
            && self.job.is_some()
            && !self.workbench.attached
            && self.permissions.is_empty()
        {
            return self.action("pause-job");
        }

        if ctrl && key.code == KeyCode::Char('q') {
            return self.action("quit");
        }
        if self.page == Page::Jobs && self.workbench.attached && self.dialog.is_none() {
            if ctrl && matches!(key.code, KeyCode::Char(']') | KeyCode::Char('5')) {
                return self.workbench_action("job-detach");
            }
            if let Some(text) = crate::jobs::key_text(key) {
                return self.terminal_input(serde_json::json!({"action":"input","text":text}));
            }
            return Ok(());
        }
        if !self.permissions.is_empty() {
            match key.code {
                KeyCode::Char('y') if ctrl => return self.permission(1),
                KeyCode::Char('n') if ctrl => return self.permission(0),
                KeyCode::Esc => return self.permission(0),
                KeyCode::Tab | KeyCode::Right => {
                    self.permission_selected = (self.permission_selected + 1) % 3
                }
                KeyCode::BackTab | KeyCode::Left => {
                    self.permission_selected = (self.permission_selected + 2) % 3
                }
                KeyCode::Enter => return self.permission(self.permission_selected),
                KeyCode::PageUp | KeyCode::Up => {
                    self.permission_scroll = self.permission_scroll.saturating_sub(4)
                }
                KeyCode::PageDown | KeyCode::Down => {
                    self.permission_scroll = self.permission_scroll.saturating_add(4)
                }
                KeyCode::Char('c') if ctrl => return self.stop_turn(),
                _ => {}
            }
            return Ok(());
        }
        if self.dialog.is_some() {
            return self.dialog_key(key);
        }
        if alt
            && let KeyCode::Char(c) = key.code
            && let Some(n) = c.to_digit(10)
            && n <= 9
        {
            self.set_page(Page::ALL[if n == 0 { 9 } else { n as usize - 1 }]);
            return Ok(());
        }
        if ctrl {
            match key.code {
                KeyCode::Char('p') => return self.action("palette"),
                KeyCode::Char('n') => return self.new_conversation(),
                KeyCode::Char('b') => {
                    self.edit_brief();
                    return Ok(());
                }
                KeyCode::Char('c') => {
                    return if self.busy || self.connecting {
                        self.stop_turn()
                    } else {
                        self.action("quit")
                    };
                }
                KeyCode::Char('t') if self.page == Page::Chat => {
                    self.tools_focus = !self.tools_focus;
                    return Ok(());
                }
                KeyCode::Char('j') if self.page == Page::Chat => {
                    self.composer.insert("\n", true);
                    return Ok(());
                }
                _ => {}
            }
        }
        if key.code == KeyCode::F(1) {
            self.set_page(Page::Help);
            return Ok(());
        }
        if key.code == KeyCode::Tab && !self.search_focus {
            self.nav_focus = !self.nav_focus;
            return Ok(());
        }
        if self.nav_focus {
            match key.code {
                KeyCode::Down => self.nav_index = (self.nav_index + 1) % Page::ALL.len(),
                KeyCode::Up => {
                    self.nav_index = (self.nav_index + Page::ALL.len() - 1) % Page::ALL.len()
                }
                KeyCode::Enter | KeyCode::Right => self.set_page(Page::ALL[self.nav_index]),
                _ => {}
            }
            return Ok(());
        }
        if self.search_focus {
            match key.code {
                KeyCode::Enter | KeyCode::Esc => self.search_focus = false,
                _ => {
                    self.session_search.key(key, false);
                    self.refresh_sessions()?;
                }
            }
            return Ok(());
        }
        match self.page {
            Page::Home => match key.code {
                KeyCode::Down => self.home_selected = (self.home_selected + 1) % 8,
                KeyCode::Up => self.home_selected = (self.home_selected + 7) % 8,
                KeyCode::Enter => self.home_action(self.home_selected)?,
                _ => {}
            },
            Page::Chat => match key.code {
                KeyCode::Esc => {
                    self.stop_turn()?;
                }
                KeyCode::Enter if alt => self.composer.insert("\n", true),
                KeyCode::Enter if self.tools_focus => {
                    self.action("tool-details")?;
                }
                KeyCode::Enter => {
                    self.send_prompt()?;
                }
                KeyCode::PageUp => self.chat_scroll = self.chat_scroll.saturating_add(8),
                KeyCode::PageDown => self.chat_scroll = self.chat_scroll.saturating_sub(8),
                KeyCode::Up if self.tools_focus => {
                    self.tool_selected = self.tool_selected.saturating_sub(1)
                }
                KeyCode::Down if self.tools_focus => {
                    self.tool_selected =
                        (self.tool_selected + 1).min(self.tools.len().saturating_sub(1))
                }
                _ if !self.tools_focus => self.composer.key(key, true),
                _ => {}
            },
            Page::Models => match key.code {
                KeyCode::Left => {
                    self.model_tab = (self.model_tab + 2) % 3;
                    self.model_selected = 0;
                }
                KeyCode::Right => {
                    self.model_tab = (self.model_tab + 1) % 3;
                    self.model_selected = 0;
                }
                KeyCode::Up => self.model_selected = self.model_selected.saturating_sub(1),
                KeyCode::Down => {
                    self.model_selected =
                        (self.model_selected + 1).min(self.model_count().saturating_sub(1))
                }
                KeyCode::Enter => self.select_model()?,
                KeyCode::Char('r') => self.refresh_models()?,
                KeyCode::Char('s') => self.action("hub-search")?,
                KeyCode::Char('i') => self.action("import")?,
                KeyCode::Char('v') => {
                    self.variants_only = !self.variants_only;
                    self.notify("Publisher-variant filter changed. Refresh to update results.");
                }
                KeyCode::Backspace => {
                    self.hub_files.clear();
                    self.model_selected = 0;
                }
                _ => {}
            },
            Page::Connections => match key.code {
                KeyCode::Down => {
                    self.connection_selected = (self.connection_selected + 1)
                        .min(self.config.profiles.len().saturating_sub(1))
                }
                KeyCode::Up => {
                    self.connection_selected = self.connection_selected.saturating_sub(1)
                }
                KeyCode::Enter => self.action("use-connection")?,
                KeyCode::Char('a') => self.connection_wizard(),
                KeyCode::Char('e') => self.edit_connection()?,
                KeyCode::Char('t') => self.action("test-connection")?,
                KeyCode::Delete => self.action("remove-connection")?,
                _ => {}
            },
            Page::Sessions => match key.code {
                KeyCode::Up => self.session_selected = self.session_selected.saturating_sub(1),
                KeyCode::Down => {
                    self.session_selected =
                        (self.session_selected + 1).min(self.sessions.len().saturating_sub(1))
                }
                KeyCode::Enter => self.action("resume")?,
                KeyCode::Char('/') => self.search_focus = true,
                KeyCode::Char('r') => self.action("rename")?,
                KeyCode::Char('e') => self.action("export")?,
                KeyCode::Char('a') => self.action("show-archived")?,
                KeyCode::Delete => self.action("archive")?,
                _ => {}
            },
            Page::Files => match key.code {
                KeyCode::Up => {
                    self.workbench.file_selected = self.workbench.file_selected.saturating_sub(1)
                }
                KeyCode::Down => {
                    self.workbench.file_selected = (self.workbench.file_selected + 1)
                        .min(self.filtered_files().len().saturating_sub(1))
                }
                KeyCode::Enter => self.workbench_action("file-view")?,
                KeyCode::Char('e') => self.workbench_action("file-edit")?,
                KeyCode::Char('n') => self.workbench_action("file-new")?,
                KeyCode::Char('/') => self.workbench_action("files-search")?,
                KeyCode::Char('s') => self.workbench_action("file-find")?,
                KeyCode::Char('g') => self.workbench_action("file-line")?,
                KeyCode::Char('r') => self.workbench_action("files-refresh")?,
                KeyCode::Char('d') => self.workbench_action("all-diffs")?,
                _ => {}
            },
            Page::Jobs => match key.code {
                KeyCode::Up => {
                    self.workbench.job_selected = self.workbench.job_selected.saturating_sub(1)
                }
                KeyCode::Down => {
                    self.workbench.job_selected = (self.workbench.job_selected + 1)
                        .min(self.workbench.jobs.len().saturating_sub(1))
                }
                KeyCode::Enter => self.workbench_action("job-attach")?,
                KeyCode::Char('n') => self.workbench_action("job-new")?,
                KeyCode::Char('s') => self.workbench_action("job-stop")?,
                KeyCode::Char('r') => self.workbench_action("job-restart")?,
                KeyCode::Char('l') => self.workbench_action("job-logs")?,
                KeyCode::Char('h') => self.workbench_action("job-health")?,
                KeyCode::F(5) => self.workbench_action("jobs-refresh")?,
                _ => {}
            },
            Page::Context => match key.code {
                KeyCode::Char('p') => self.workbench_action("pin-requirement")?,
                KeyCode::Char('u') => self.workbench_action("unpin-requirement")?,
                KeyCode::Char('r') => self.workbench_action("context-refresh")?,
                KeyCode::PageDown => self.help_scroll = self.help_scroll.saturating_add(8),
                KeyCode::PageUp => self.help_scroll = self.help_scroll.saturating_sub(8),
                _ => {}
            },
            Page::Task => match key.code {
                KeyCode::Char('v') => self.workbench_action("verification-status")?,
                KeyCode::Char('a') => self.workbench_action("verification-plan")?,
                KeyCode::Char('t') => self.workbench_action("verification-run")?,
                KeyCode::Up => self.change_selected = self.change_selected.saturating_sub(1),
                KeyCode::Down => {
                    self.change_selected = (self.change_selected + 1)
                        .min(self.task_view.changes.len().saturating_sub(1))
                }
                KeyCode::Enter => self.task_action("task-diff")?,
                KeyCode::Char('r') => self.task_action("task-check")?,
                KeyCode::Char('u') => self.task_action("task-undo")?,
                KeyCode::Char('U') => self.task_action("task-undo-all")?,
                KeyCode::Char('x') => self.task_action("task-export")?,
                KeyCode::Char('c') => self.task_action("task-configure")?,
                KeyCode::Char('m') => self.task_action("task-memory")?,
                KeyCode::Char('e') => self.task_action("task-evidence")?,
                KeyCode::Char('n') => self.task_action("task-note")?,
                KeyCode::Esc => self.action("pause-job")?,
                _ => {}
            },
            Page::Settings => match key.code {
                KeyCode::Up => self.settings_selected = self.settings_selected.saturating_sub(1),
                KeyCode::Down => self.settings_selected = (self.settings_selected + 1).min(16),
                KeyCode::Enter | KeyCode::Right => self.setting_action(self.settings_selected)?,
                _ => {}
            },
            Page::Help => match key.code {
                KeyCode::PageDown | KeyCode::Down => {
                    self.help_scroll = self.help_scroll.saturating_add(4)
                }
                KeyCode::PageUp | KeyCode::Up => {
                    self.help_scroll = self.help_scroll.saturating_sub(4)
                }
                _ => {}
            },
        }
        Ok(())
    }

    pub fn model_count(&self) -> usize {
        match self.model_tab {
            0 => self.artifacts.len(),
            1 => self.server_models.len(),
            _ => {
                if self.hub_files.is_empty() {
                    self.hub_models.len()
                } else {
                    self.hub_files.len()
                }
            }
        }
    }

    pub fn home_action(&mut self, index: usize) -> Result<()> {
        match index {
            0 => {
                if self.config.profiles.is_empty() {
                    self.connection_wizard();
                } else {
                    self.new_conversation()?;
                }
            }
            2 => self.open_browser(BrowserKind::Project, self.preferences.project.clone())?,
            4 => {
                self.set_page(Page::Connections);
                if self.config.profiles.is_empty() {
                    self.connection_wizard();
                }
            }
            5 => self.action("templates")?,
            6 => {
                if self.engine_ready() {
                    self.notify("The agent engine is installed. You can start a conversation.");
                } else {
                    self.install_dialog(Component::Engine);
                }
            }
            7 => self.edit_brief(),
            1 => {
                if self
                    .preferences
                    .project
                    .parent()
                    .is_some_and(|p| p.join("lesson.json").is_file())
                {
                    self.practice_guide();
                } else {
                    self.new_practice()?;
                }
            }
            3 => self.task_action("task-configure")?,
            _ => {}
        }
        Ok(())
    }

    pub fn action(&mut self, action: &str) -> Result<()> {
        if action.starts_with("file")
            || action.starts_with("job-")
            || action.starts_with("jobs-")
            || action.starts_with("verification-")
            || matches!(
                action,
                "all-diffs" | "context-refresh" | "pin-requirement" | "unpin-requirement"
            )
        {
            return self.workbench_action(action);
        }

        if action.starts_with("task") || matches!(action, "access" | "hardware" | "evaluate") {
            return self.task_action(action);
        }
        match action {
            "quit" => {
                if self.busy || self.connecting || self.job.is_some() {
                    self.dialog=Some(Dialog::Confirm{title:"Leave Alt?".into(),body:"Your conversation is saved. The active task and any download will stop; partial downloads can be resumed later.".into(),action:ConfirmAction::Quit,selected:0});
                } else {
                    self.quit = true;
                }
            }
            "new" => self.new_conversation()?,
            "connect" => self.connection_wizard(),
            "send" => self.send_prompt()?,
            "stop" => self.stop_turn()?,
            "reconnect" => self.start_workspace(self.session.clone())?,
            "allowance" => self.manager_action("inference-live", serde_json::json!({}))?,
            "brief" => self.edit_brief(),
            "project" => {
                self.open_browser(BrowserKind::Project, self.preferences.project.clone())?
            }
            "templates" => {
                self.dialog = Some(Dialog::Menu {
                    title: "What would you like to do?".into(),
                    description:
                        "Choose a starting point. You can edit the message before sending it."
                            .into(),
                    items: TEMPLATES
                        .iter()
                        .enumerate()
                        .map(|(i, (label, hint, _))| {
                            (label.to_string(), hint.to_string(), MenuAction::Template(i))
                        })
                        .collect(),
                    selected: 0,
                })
            }
            "palette" => {
                let mut items = Page::ALL
                    .iter()
                    .map(|p| {
                        (
                            p.name().to_string(),
                            "Open page".into(),
                            MenuAction::Page(*p),
                        )
                    })
                    .collect::<Vec<_>>();
                items.extend([
                    (
                        "New conversation".into(),
                        "Keep previous conversations saved".into(),
                        MenuAction::New,
                    ),
                    (
                        "Choose project".into(),
                        "Browse folders".into(),
                        MenuAction::Project,
                    ),
                    (
                        "Edit project brief".into(),
                        "Keep goals and decisions in context".into(),
                        MenuAction::Brief,
                    ),
                ]);
                self.dialog = Some(Dialog::Menu {
                    title: "Quick actions".into(),
                    description: "Arrow keys to choose; Enter to open.".into(),
                    items,
                    selected: 0,
                });
            }
            "tool-details" => {
                let tool = self
                    .tools
                    .get(self.tool_selected)
                    .context("No tool activity yet")?
                    .clone();
                self.dialog = Some(Dialog::ToolDetails { tool, scroll: 0 });
            }
            "refresh-models" => self.refresh_models()?,
            "use-model" => self.select_model()?,
            "hub-search" => {
                self.dialog=Some(Dialog::Input{title:"Find a model on Hugging Face".into(),hint:"Try Qwen3 heretic, Spark, or MiMo. The variant filter uses publisher names/tags, not a behavior test.".into(),editor:Editor::new(&self.hub_query),action:InputAction::SearchHub,multiline:false});
            }
            "hub-repo" => {
                self.dialog=Some(Dialog::Input{title:"Open a model repository".into(),hint:"Enter publisher/model-name. Only complete GGUF files with published checksums are offered.".into(),editor:Editor::default(),action:InputAction::DirectRepo,multiline:false});
            }
            "hub-back" => {
                self.hub_files.clear();
                self.model_selected = 0;
            }
            "variant-filter" => {
                self.variants_only = !self.variants_only;
                self.notify("Filter changed. Refresh the search to apply it.");
            }
            "import" => self.open_browser(BrowserKind::Model, self.preferences.project.clone())?,
            "runtime" => {
                if self.runtime_ready() {
                    self.notify("The local CPU runtime is installed. Select a local model and start a conversation.");
                } else {
                    self.install_dialog(Component::Inference);
                }
            }
            "pause-job" => {
                if let Some(job) = &mut self.job {
                    job.cancel.store(true, Ordering::Relaxed);
                    job.progress.stage = "Stop requested; finishing the current atomic operation. Downloads remain resumable.".into();
                }
            }
            "use-connection" => {
                let name = self
                    .config
                    .profiles
                    .keys()
                    .nth(self.connection_selected)
                    .context("Add a connection first")?
                    .clone();
                self.config.default_profile = name;
                self.config.save(&self.root)?;
                self.notify("Connection selected for new conversations.");
            }
            "edit-connection" => self.edit_connection()?,
            "test-connection" => {
                let (name, profile) = self
                    .config
                    .profiles
                    .iter()
                    .nth(self.connection_selected)
                    .context("Add a connection first")?;
                ensure!(
                    profile.local_model.is_none(),
                    "Start a conversation to load and check this local model"
                );
                let profile = profile.clone();
                let name = name.clone();
                self.model_tab = 1;
                self.set_page(Page::Models);
                self.launch_job("Testing connection", move |_, _| async move {
                    Ok(JobResult::Inventory(
                        name,
                        models::inventory(&profile).await?,
                    ))
                })?;
            }
            "remove-connection" => {
                let name = self
                    .config
                    .profiles
                    .keys()
                    .nth(self.connection_selected)
                    .context("No connection selected")?
                    .clone();
                self.dialog = Some(Dialog::Confirm {
                    title: "Remove this connection?".into(),
                    body: format!(
                        "{name}\n\nThis removes its saved settings. Your models, project files, and conversations are kept."
                    ),
                    action: ConfirmAction::RemoveConnection(name),
                    selected: 0,
                });
            }
            "resume" => {
                let id = self
                    .sessions
                    .get(self.session_selected)
                    .context("No conversation selected")?
                    .id
                    .clone();
                self.resume_workspace(id)?;
            }
            "rename" => {
                let session = self
                    .sessions
                    .get(self.session_selected)
                    .context("No conversation selected")?;
                self.dialog = Some(Dialog::Input {
                    title: "Name this conversation".into(),
                    hint: "Use a short title that will help you find it later.".into(),
                    editor: Editor::new(&session.title),
                    action: InputAction::Rename(session.id.clone()),
                    multiline: false,
                });
            }
            "export" => {
                let id = if self.page == Page::Sessions {
                    self.sessions
                        .get(self.session_selected)
                        .map(|s| s.id.clone())
                } else {
                    self.session.as_ref().map(|s| s.id.clone())
                }
                .context("No conversation to export yet")?;
                let root = self.root.clone();
                self.launch_io("Exporting conversation",move || {
                    let path=crate::store::Store::open(&root)?.export_files(&root,&id)?;
                    Ok(JobResult::Notice("Conversation exported".into(),format!("Saved Markdown and full JSONL evidence. Review project data before sharing.\n{}",path.display())))
                })?;
            }
            "show-archived" => {
                self.archived = !self.archived;
                self.refresh_sessions()?;
            }
            "archive" => {
                let session = self
                    .sessions
                    .get(self.session_selected)
                    .context("No conversation selected")?;
                self.dialog = Some(Dialog::Confirm {
                    title: if self.archived {
                        "Restore conversation?"
                    } else {
                        "Archive conversation?"
                    }
                    .into(),
                    body: format!(
                        "{}\n\nYou can restore archived conversations at any time. Project files and model weights are unaffected.",
                        session.title
                    ),
                    action: ConfirmAction::Archive(session.id.clone(), !self.archived),
                    selected: 0,
                });
            }
            "search-sessions" => self.search_focus = true,
            "choose-folder" => {
                if let Some(Dialog::Browser {
                    kind: BrowserKind::Project,
                    path,
                    ..
                }) = &self.dialog
                {
                    let path = path.clone();
                    self.choose_project(&path)?;
                }
            }
            "type-path" => {
                let (kind, path) = if let Some(Dialog::Browser { kind, path, .. }) = &self.dialog {
                    (*kind, path.clone())
                } else {
                    return Ok(());
                };
                let action = match kind {
                    BrowserKind::Project => InputAction::ProjectPath,
                    BrowserKind::Model => InputAction::ImportPath,
                    BrowserKind::Engine => InputAction::EnginePath,
                    BrowserKind::Runtime => InputAction::RuntimePath,
                };
                self.dialog = Some(Dialog::Input {
                    title: "Enter a path".into(),
                    hint: "You can paste an absolute path. ~/ is supported.".into(),
                    editor: Editor::new(path.to_string_lossy()),
                    action,
                    multiline: false,
                });
            }
            "modal-cancel" => {
                self.dialog = None;
            }
            "modal-submit" => {
                return self.dialog_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
            }
            "save-brief" => {
                return self.dialog_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
            }
            "test-form" => return self.test_form(),
            _ => {}
        }
        Ok(())
    }

    pub fn setting_action(&mut self, index: usize) -> Result<()> {
        match index {
            0 => {
                let values = [4096, 8192, 16384, 32768];
                let current = self
                    .current_profile()
                    .map(|(_, p)| p.context_tokens)
                    .unwrap_or(self.preferences.context_tokens);
                let n = values.iter().position(|v| *v == current).unwrap_or(0);
                self.preferences.context_tokens = values[(n + 1) % 4];
            }
            1 => {
                let values = [6, 12, 24];
                let current = self
                    .current_profile()
                    .map(|(_, p)| p.max_turns)
                    .unwrap_or(self.preferences.max_turns);
                let n = values.iter().position(|v| *v == current).unwrap_or(0);
                self.preferences.max_turns = values[(n + 1) % 3];
            }
            2 => {
                self.preferences.mouse = !self.preferences.mouse;
                if self.preferences.mouse {
                    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
                } else {
                    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;
                }
            }
            3 => {
                self.open_browser(BrowserKind::Engine, self.preferences.project.clone())?;
                return Ok(());
            }
            4 => {
                self.open_browser(BrowserKind::Runtime, self.preferences.project.clone())?;
                return Ok(());
            }
            13 => return self.manager_action("runtime-settings", serde_json::json!({})),
            14 => return self.manager_action("benchmark-preview", serde_json::json!({})),
            15 => return self.manager_action("tools-focus", serde_json::json!({})),
            16 => return self.manager_action("qualify-preview", serde_json::json!({})),
            10 => return self.manager_action("manage-tools", serde_json::json!({})),
            11 => return self.manager_action("manage-storage", serde_json::json!({})),
            12 => return self.manager_action("manage-models", serde_json::json!({})),
            6 => return self.task_action("access"),
            7 => return self.task_action("hardware"),
            8 => return self.task_action("evaluate"),
            9 => {
                let profile = self
                    .config
                    .profiles
                    .get_mut(&self.config.default_profile)
                    .context("Choose a model first")?;
                profile.uncensored = !profile.uncensored;
                self.config.save(&self.root)?;
                self.notify("Publisher/user model claim updated. This does not establish its quality or refusal behavior.");
                return Ok(());
            }
            5 => {
                self.edit_brief();
                return Ok(());
            }
            _ => {}
        }
        if index < 2
            && let Some(profile) = self.config.profiles.get_mut(&self.config.default_profile)
        {
            profile.context_tokens = self.preferences.context_tokens;
            profile.max_turns = self.preferences.max_turns;
            self.config.save(&self.root)?;
        }
        self.preferences.save(&self.root)?;
        self.notify("Settings saved. Context and step limits apply to new conversations.");
        Ok(())
    }

    fn test_form(&mut self) -> Result<()> {
        let Some(Dialog::Connection(form)) = &self.dialog else {
            return Ok(());
        };
        let draft = self.draft(form)?;
        self.launch_job("Checking the connection", move |_, _| async move {
            let models = models::inventory(&draft.profile).await?;
            Ok(JobResult::Tested(draft, models))
        })
    }
    fn draft(&self, form: &ConnectionForm) -> Result<ConnectionDraft> {
        let key = form.fields[2].text.trim();
        let original = form.original.clone();
        let profile = Profile {
            provider: form.provider,
            endpoint: form.fields[1].text.trim().trim_end_matches('/').into(),
            model: "choose-next".into(),
            context_tokens: self.preferences.context_tokens,
            max_turns: self.preferences.max_turns,
            uncensored: false,
            api_key_env: (!key.is_empty()).then(|| key.into()),
            local_model: None,
            inference: None,
        };
        profile.validate()?;
        ensure!(
            !form.fields[0].text.trim().is_empty(),
            "Give your connection a name"
        );
        Ok(ConnectionDraft {
            name: form.fields[0].text.trim().into(),
            profile,
            original,
        })
    }

    fn dialog_key(&mut self, key: KeyEvent) -> Result<()> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let Some(mut dialog) = self.dialog.take() else {
            return Ok(());
        };
        if key.code == KeyCode::Esc {
            if matches!(dialog, Dialog::Notice { .. }) {
                self.dialog = self.recovery_dialog.take().map(|d| *d);
            }
            return Ok(());
        }
        // Keep a recoverable form on validation errors.
        self.dialog = Some(dialog.clone());
        match &mut dialog {
            Dialog::Notice { scroll, .. } => {
                match key.code {
                    KeyCode::PageDown => *scroll = scroll.saturating_add(8),
                    KeyCode::PageUp => *scroll = scroll.saturating_sub(8),
                    KeyCode::Down => *scroll = scroll.saturating_add(4),
                    KeyCode::Up => *scroll = scroll.saturating_sub(4),
                    _ => {}
                }
                if key.code == KeyCode::Enter {
                    self.dialog = self.recovery_dialog.take().map(|d| *d);
                    return Ok(());
                }
            }
            Dialog::Menu {
                items, selected, ..
            } => match key.code {
                KeyCode::Down => *selected = (*selected + 1) % items.len().max(1),
                KeyCode::Up => {
                    *selected = (*selected + items.len().saturating_sub(1)) % items.len().max(1)
                }
                KeyCode::Enter => {
                    if let Some((_, _, action)) = items.get(*selected) {
                        return self.menu_action(action.clone());
                    }
                }
                _ => {}
            },
            Dialog::Connection(form) => match key.code {
                KeyCode::Tab | KeyCode::Down => form.focus = (form.focus + 1) % 3,
                KeyCode::BackTab | KeyCode::Up => form.focus = (form.focus + 2) % 3,
                KeyCode::Enter => return self.test_form(),
                KeyCode::Char('m') if ctrl => {
                    let draft = self.draft(form)?;
                    self.dialog=Some(Dialog::Input{title:"Enter a model identifier".into(),hint:"Use the exact name from your server. This saves without checking its inventory.".into(),editor:Editor::default(),action:InputAction::ManualModel(draft),multiline:false});
                    return Ok(());
                }
                _ => form.fields[form.focus].key(key, false),
            },
            Dialog::PickModel {
                draft,
                models,
                selected,
            } => match key.code {
                KeyCode::Up => *selected = selected.saturating_sub(1),
                KeyCode::Down => *selected = (*selected + 1).min(models.len().saturating_sub(1)),
                KeyCode::Enter => {
                    let mut draft = draft.clone();
                    draft.profile.model = models.get(*selected).context("Select a model")?.clone();
                    return self.save_connection(draft);
                }
                _ => {}
            },
            Dialog::Input {
                editor,
                action,
                multiline,
                ..
            } => {
                if key.code == KeyCode::Enter && !*multiline
                    || ctrl && key.code == KeyCode::Char('s')
                {
                    return self.submit_input(action.clone(), editor.text.clone());
                }
                editor.key(key, *multiline);
            }
            Dialog::Browser {
                kind,
                path,
                entries,
                selected,
            } => match key.code {
                KeyCode::Up => *selected = selected.saturating_sub(1),
                KeyCode::Down => *selected = (*selected + 1).min(entries.len().saturating_sub(1)),
                KeyCode::Char('s') if ctrl && matches!(kind, BrowserKind::Project) => {
                    return self.choose_project(path);
                }
                KeyCode::Char('/') => return self.action("type-path"),
                KeyCode::Backspace => {
                    if let Some(parent) = path.parent() {
                        return self.open_browser(*kind, parent.to_path_buf());
                    }
                }
                KeyCode::Enter => {
                    if let Some(entry) = entries.get(*selected) {
                        if entry.is_dir() {
                            return self.open_browser(*kind, entry.clone());
                        }
                        return match kind {
                            BrowserKind::Model => self.import_path(entry.clone()),
                            BrowserKind::Engine => self.submit_input(
                                InputAction::EnginePath,
                                entry.to_string_lossy().into(),
                            ),
                            BrowserKind::Runtime => self.submit_input(
                                InputAction::RuntimePath,
                                entry.to_string_lossy().into(),
                            ),
                            BrowserKind::Project => Ok(()),
                        };
                    }
                }
                _ => {}
            },
            Dialog::Confirm {
                action, selected, ..
            } => match key.code {
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Right => {
                    *selected = 1 - *selected
                }
                KeyCode::Enter => {
                    if *selected == 0 {
                        self.dialog = None;
                        return Ok(());
                    }
                    return self.confirm(action.clone());
                }
                _ => {}
            },
            Dialog::ToolDetails { scroll, .. } => match key.code {
                KeyCode::PageUp | KeyCode::Up => *scroll = scroll.saturating_sub(5),
                KeyCode::PageDown | KeyCode::Down => *scroll = scroll.saturating_add(5),
                KeyCode::Enter => {
                    self.dialog = None;
                    return Ok(());
                }
                _ => {}
            },
        }
        self.dialog = Some(dialog);
        Ok(())
    }

    pub fn paste(&mut self, text: &str) {
        if self.page == Page::Jobs && self.workbench.attached {
            if let Err(e) = self.terminal_input(serde_json::json!({"action":"input","text":text})) {
                self.error(e);
            }
            return;
        }

        if !self.permissions.is_empty() {
            return;
        }
        match &mut self.dialog {
            Some(Dialog::Connection(form)) => form.fields[form.focus].insert(text, false),
            Some(Dialog::Input {
                editor, multiline, ..
            }) => editor.insert(text, *multiline),
            None if self.page == Page::Chat => self.composer.insert(text, true),
            None if self.search_focus => self.session_search.insert(text, false),
            _ => {}
        }
    }

    pub fn mouse(&mut self, kind: MouseEventKind, x: u16, y: u16) -> Result<()> {
        if matches!(kind, MouseEventKind::ScrollUp | MouseEventKind::ScrollDown) {
            return self.key(KeyEvent::new(
                if kind == MouseEventKind::ScrollUp {
                    KeyCode::PageUp
                } else {
                    KeyCode::PageDown
                },
                KeyModifiers::NONE,
            ));
        }
        if kind != MouseEventKind::Down(MouseButton::Left) {
            return Ok(());
        }
        let hit = self
            .hits
            .iter()
            .rev()
            .find(|(rect, _)| rect.contains((x, y).into()))
            .map(|(_, hit)| hit.clone());
        match hit {
            Some(Hit::Nav(page)) => self.set_page(page),
            Some(Hit::Home(index)) => {
                self.home_selected = index;
                self.home_action(index)?;
            }
            Some(Hit::Button(action)) => self.action(action)?,
            Some(Hit::ModelTab(tab)) => {
                self.model_tab = tab;
                self.model_selected = 0;
            }
            Some(Hit::Field(index)) => {
                if let Some(Dialog::Connection(form)) = &mut self.dialog {
                    form.focus = index;
                }
            }
            Some(Hit::Choice(choice)) => {
                if !self.permissions.is_empty() {
                    self.permission(choice)?;
                } else if let Some(Dialog::Confirm { selected, .. }) = &mut self.dialog {
                    *selected = choice;
                    self.dialog_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?;
                } else if let Some(Dialog::Menu { selected, .. }) = &mut self.dialog {
                    *selected = choice;
                    self.dialog_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))?;
                }
            }
            Some(Hit::Row(index)) => match &mut self.dialog {
                Some(Dialog::Browser { selected, .. })
                | Some(Dialog::PickModel { selected, .. }) => *selected = index,
                _ => match self.page {
                    Page::Files => self.workbench.file_selected = index,
                    Page::Jobs => self.workbench.job_selected = index,
                    Page::Task => self.change_selected = index,
                    Page::Models => self.model_selected = index,
                    Page::Connections => self.connection_selected = index,
                    Page::Sessions => self.session_selected = index,
                    Page::Settings => {
                        self.settings_selected = index;
                        self.setting_action(index)?;
                    }
                    Page::Chat => {
                        self.tool_selected = index;
                        self.tools_focus = true;
                    }
                    _ => {}
                },
            },
            Some(Hit::Composer) => {
                self.nav_focus = false;
                self.tools_focus = false;
            }
            _ => {}
        }
        Ok(())
    }
}
