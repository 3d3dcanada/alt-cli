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
        if let Some(action) = super::actions::shortcut(key) {
            return self.action(action);
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
        if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) && !self.search_focus {
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
                KeyCode::Left | KeyCode::Right => {
                    self.move_home_horizontally(key.code == KeyCode::Right)
                }
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
                KeyCode::Char('i') => self.understanding()?,
                KeyCode::Char('c') => self.correct_requirement()?,
                KeyCode::Char('p') => self.workbench_action("pin-requirement")?,
                KeyCode::Char('u') => self.workbench_action("unpin-requirement")?,
                KeyCode::Char('r') => self.workbench_action("context-refresh")?,
                KeyCode::PageDown => self.help_scroll = self.help_scroll.saturating_add(8),
                KeyCode::PageUp => self.help_scroll = self.help_scroll.saturating_sub(8),
                _ => {}
            },
            Page::Task => match key.code {
                KeyCode::Char('l') => self.manager_action("check-logs", serde_json::json!({}))?,
                KeyCode::Char('i') => self.understanding()?,
                KeyCode::Char('f') => self.recovered_files()?,
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
                KeyCode::Down => self.settings_selected = (self.settings_selected + 1).min(17),
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

    fn move_home_horizontally(&mut self, right: bool) {
        let Some((current, _)) = self
            .hits
            .iter()
            .find(|(_, hit)| matches!(hit, Hit::Home(index) if *index == self.home_selected))
        else {
            return;
        };
        // Follow the rendered cards so changing layout or terminal size cannot
        // send selection into an invisible second column. The selected hint at
        // the bottom is a duplicate hit, after the actual cards/list rows.
        if let Some(index) = self.hits.iter().find_map(|(rect, hit)| {
            let Hit::Home(index) = hit else { return None };
            (rect.y == current.y
                && if right {
                    rect.x > current.x
                } else {
                    rect.x < current.x
                })
            .then_some(*index)
        }) {
            self.home_selected = index;
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
        let page = match action {
            "page-home" => Some(Page::Home),
            "page-chat" => Some(Page::Chat),
            "page-models" => Some(Page::Models),
            "page-connections" => Some(Page::Connections),
            "page-sessions" => Some(Page::Sessions),
            "page-settings" => Some(Page::Settings),
            "page-help" => Some(Page::Help),
            "page-files" => Some(Page::Files),
            "page-jobs" => Some(Page::Jobs),
            "page-context" => Some(Page::Context),
            _ => None,
        };
        if let Some(page) = page {
            self.set_page(page);
            return Ok(());
        }

        if action.starts_with("file")
            || action.starts_with("job-")
            || action.starts_with("jobs-")
            || action.starts_with("verification-")
            || matches!(
                action,
                "all-diffs" | "context-refresh" | "pin-requirement" | "unpin-requirement"
            )
        {
            if action == "files-search" {
                self.set_page(Page::Files);
            }
            self.workbench_action(action)?;
            if action == "job-attach" {
                // A palette action can start outside the Jobs page. Keep the
                // terminal receiving input visible after attachment succeeds.
                self.page = Page::Jobs;
                self.nav_index = Page::ALL.iter().position(|p| *p == Page::Jobs).unwrap();
                self.nav_focus = false;
            }
            return Ok(());
        }

        if action.starts_with("task") || matches!(action, "access" | "hardware" | "evaluate") {
            return self.task_action(action);
        }
        match action {
            "appearance" => self.appearance_dialog(),
            "quit" => {
                if self.busy || self.connecting || self.job.is_some() {
                    self.dialog=Some(Dialog::Confirm{title:"Leave Alt?".into(),body:"Your conversation is saved. The active task and any download will stop; partial downloads can be resumed later.".into(),action:ConfirmAction::Quit,selected:0});
                } else {
                    self.quit = true;
                }
            }
            "new" => self.new_conversation()?,
            "practice" => self.home_action(1)?,
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
            "model-fit" | "storage-usage" | "storage-budget" | "inference-export"
            | "check-inputs" | "check-logs" | "manage-tools" | "manage-storage"
            | "manage-models" | "runtime-settings" | "benchmark-preview" | "tools-focus"
            | "qualify-preview" => self.manager_action(action, serde_json::json!({}))?,
            "palette" => {
                self.dialog = Some(Dialog::Palette {
                    query: Editor::default(),
                    selected: 0,
                })
            }
            "drafts" => self.draft_menu(),
            "save-drafts" => {
                self.remember_draft();
                self.flush_drafts = true;
            }
            "manual-model" => self.manual_model()?,
            "understanding" => self.understanding()?,
            "correct-requirement" => self.correct_requirement()?,
            "recovered-files" => self.recovered_files()?,
            "next-step" => {
                let (_, _, action) = self.next_step();
                self.action(action)?;
            }
            "tool-details" => {
                let tool = self
                    .tools
                    .get(self.tool_selected)
                    .context("No tool activity yet")?
                    .clone();
                self.dialog = Some(Dialog::ToolDetails { tool, scroll: 0 });
            }
            "refresh-models" => {
                self.set_page(Page::Models);
                self.refresh_models()?;
            }
            "use-model" => self.select_model()?,
            "hub-search" => {
                self.set_page(Page::Models);
                self.model_tab = 2;
                self.dialog=Some(Dialog::Input{title:"Find a model on Hugging Face".into(),hint:"Try Qwen3 heretic, Spark, or MiMo. The variant filter uses publisher names/tags, not a behavior test.".into(),editor:Editor::new(&self.hub_query),action:InputAction::SearchHub,multiline:false});
            }
            "hub-repo" => {
                self.set_page(Page::Models);
                self.model_tab = 2;
                self.dialog=Some(Dialog::Input{title:"Open a model repository".into(),hint:"Enter publisher/model-name. Only complete GGUF files with published checksums are offered.".into(),editor:Editor::default(),action:InputAction::DirectRepo,multiline:false});
            }
            "hub-back" => {
                self.set_page(Page::Models);
                self.model_tab = 2;
                self.hub_files.clear();
                self.model_selected = 0;
            }
            "variant-filter" => {
                self.set_page(Page::Models);
                self.model_tab = 2;
                self.variants_only = !self.variants_only;
                self.notify("Filter changed. Refresh the search to apply it.");
            }
            "import" => self.open_browser(BrowserKind::Model, self.preferences.project.clone())?,
            "install-engine" => self.install_dialog(Component::Engine),
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
                self.launch_job("Testing connection", move |cancel, _| async move {
                    let models = tokio::select! { result = models::inventory(&profile) => result?, _ = models::cancelled(&cancel) => anyhow::bail!("Connection check cancelled") };
                    Ok(JobResult::Inventory(name, models))
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
                self.set_page(Page::Sessions);
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
            "search-sessions" => {
                self.set_page(Page::Sessions);
                self.search_focus = true;
            }
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
                if matches!(self.dialog, Some(Dialog::Connection(_)))
                    && let Some(job) = &self.job
                {
                    job.cancel.store(true, Ordering::Relaxed);
                }
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
            17 => {
                self.appearance_dialog();
                return Ok(());
            }
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
        self.status_error = false;
        self.launch_job("Checking the connection", move |cancel, _| async move {
            let models = tokio::select! { result = models::inventory(&draft.profile) => result?, _ = models::cancelled(&cancel) => anyhow::bail!("Connection check cancelled") };
            Ok(JobResult::Tested(draft, models))
        })
    }
    pub(super) fn draft(&self, form: &ConnectionForm) -> Result<ConnectionDraft> {
        let key = form.fields[2].text.trim();
        let original = form.original.clone();
        let mut profile = form
            .original
            .as_ref()
            .and_then(|name| self.config.profiles.get(name))
            .cloned()
            .unwrap_or(Profile {
                provider: form.provider,
                endpoint: String::new(),
                model: "choose-next".into(),
                context_tokens: self.preferences.context_tokens,
                max_turns: self.preferences.max_turns,
                uncensored: false,
                api_key_env: None,
                local_model: None,
                inference: None,
            });
        // A connection form is a typed patch: unrelated allocations and claims survive.
        profile.endpoint = form.fields[1].text.trim().trim_end_matches('/').into();
        profile.api_key_env = (!key.is_empty()).then(|| key.into());
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
            if matches!(dialog, Dialog::Connection(_))
                && let Some(job) = &self.job
            {
                job.cancel.store(true, Ordering::Relaxed);
            }
            if matches!(dialog, Dialog::Notice { .. }) {
                self.dialog = self.recovery_dialog.take().map(|d| *d);
            }
            return Ok(());
        }
        // Keep a recoverable form on validation errors.
        self.dialog = Some(dialog.clone());
        match &mut dialog {
            Dialog::Palette { query, selected } => {
                let entries = super::actions::search(&query.text);
                match key.code {
                    KeyCode::Down => {
                        *selected = (*selected + 1).min(entries.len().saturating_sub(1))
                    }
                    KeyCode::Up => *selected = selected.saturating_sub(1),
                    KeyCode::Enter => {
                        if let Some(action) = entries.get(*selected) {
                            self.dialog = None;
                            return self.action(action.id);
                        }
                    }
                    _ => {
                        query.key(key, false);
                        *selected = 0;
                    }
                }
            }
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
                KeyCode::Char('o') if ctrl => return self.manual_model(),
                _ => {
                    if let Some(job) = &self.job
                        && job.label == "Checking the connection"
                    {
                        job.cancel.store(true, Ordering::Relaxed);
                    }
                    form.fields[form.focus].key(key, false);
                }
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
                KeyCode::PageUp | KeyCode::Up => {
                    self.modal_scroll = self.modal_scroll.saturating_sub(4)
                }
                KeyCode::PageDown | KeyCode::Down => {
                    self.modal_scroll = self.modal_scroll.saturating_add(4)
                }
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
        if matches!(self.dialog, Some(Dialog::Connection(_)))
            && let Some(job) = &self.job
            && job.label == "Checking the connection"
        {
            job.cancel.store(true, Ordering::Relaxed);
        }
        let incoming = text
            .chars()
            .filter(|c| *c == '\n' || *c == '\t' || !c.is_control())
            .map(char::len_utf8)
            .sum::<usize>();
        let existing = match &self.dialog {
            Some(Dialog::Connection(form)) => Some(form.fields[form.focus].text.len()),
            Some(Dialog::Input { editor, .. }) => Some(editor.text.len()),
            Some(Dialog::Palette { query, .. }) => Some(query.text.len()),
            None if self.page == Page::Chat => Some(self.composer.text.len()),
            None if self.search_focus => Some(self.session_search.text.len()),
            _ => None,
        };
        if existing.is_some_and(|len| len.saturating_add(incoming) > 64 * 1024) {
            self.error("Paste exceeds the 64 KiB editor limit. Existing text is unchanged. Put the large text in a project file and include its path in your request, or paste a smaller section.");
            return;
        }
        match &mut self.dialog {
            Some(Dialog::Palette { query, selected }) => {
                query.insert(text, false);
                *selected = 0;
            }
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
            return self.scroll_mouse(kind == MouseEventKind::ScrollDown);
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
                } else if let Some(
                    Dialog::Menu { selected, .. } | Dialog::Palette { selected, .. },
                ) = &mut self.dialog
                {
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

    /// Lists use their selection as the viewport anchor. Wheel movement must not
    /// activate a setting, wrap at the end, or type cursor keys into a form.
    fn scroll_mouse(&mut self, down: bool) -> Result<()> {
        fn move_selection(selected: &mut usize, count: usize, down: bool) {
            *selected = if down {
                selected.saturating_add(3).min(count.saturating_sub(1))
            } else {
                selected.saturating_sub(3)
            };
        }
        let page_key = if down {
            KeyCode::PageDown
        } else {
            KeyCode::PageUp
        };
        if !self.permissions.is_empty() {
            return self.key(KeyEvent::new(page_key, KeyModifiers::NONE));
        }
        match &mut self.dialog {
            Some(Dialog::Menu {
                items, selected, ..
            }) => {
                move_selection(selected, items.len(), down);
                return Ok(());
            }
            Some(Dialog::Palette { query, selected }) => {
                move_selection(selected, super::actions::search(&query.text).len(), down);
                return Ok(());
            }
            Some(Dialog::Browser {
                entries, selected, ..
            }) => {
                move_selection(selected, entries.len(), down);
                return Ok(());
            }
            Some(Dialog::PickModel {
                models, selected, ..
            }) => {
                move_selection(selected, models.len(), down);
                return Ok(());
            }
            Some(Dialog::Connection(_) | Dialog::Input { .. }) => return Ok(()),
            Some(_) => return self.key(KeyEvent::new(page_key, KeyModifiers::NONE)),
            None => {}
        }
        if self.nav_focus {
            move_selection(&mut self.nav_index, Page::ALL.len(), down);
            return Ok(());
        }
        match self.page {
            Page::Home => move_selection(&mut self.home_selected, 8, down),
            Page::Models => {
                let count = self.model_count();
                move_selection(&mut self.model_selected, count, down);
            }
            Page::Connections => move_selection(
                &mut self.connection_selected,
                self.config.profiles.len(),
                down,
            ),
            Page::Sessions => move_selection(&mut self.session_selected, self.sessions.len(), down),
            Page::Settings => move_selection(&mut self.settings_selected, 18, down),
            Page::Files => {
                let count = self.filtered_files().len();
                move_selection(&mut self.workbench.file_selected, count, down);
            }
            Page::Task => move_selection(
                &mut self.change_selected,
                self.task_view.changes.len(),
                down,
            ),
            Page::Jobs if !self.workbench.attached => move_selection(
                &mut self.workbench.job_selected,
                self.workbench.jobs.len(),
                down,
            ),
            Page::Chat if self.tools_focus => {
                move_selection(&mut self.tool_selected, self.tools.len(), down)
            }
            _ => return self.key(KeyEvent::new(page_key, KeyModifiers::NONE)),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Preferences;

    fn setup() -> (tempfile::TempDir, App) {
        let directory = tempfile::tempdir().unwrap();
        let mut app = App::load(directory.path().join("state"), "goose".into(), None).unwrap();
        app.preferences.project = directory.path().into();
        (directory, app)
    }

    fn palette(app: &mut App, query: &str) {
        app.action("palette").unwrap();
        app.paste(query);
        app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .unwrap();
    }

    #[test]
    fn home_arrows_cross_visible_card_columns_without_activating_them() {
        let (_directory, mut app) = setup();
        app.hits = (0..8)
            .map(|index| {
                (
                    ratatui::layout::Rect::new(
                        if index < 4 { 3 } else { 50 },
                        10 + (index % 4) as u16 * 4,
                        44,
                        4,
                    ),
                    Hit::Home(index),
                )
            })
            .collect();
        app.hits
            .push((ratatui::layout::Rect::new(3, 30, 90, 2), Hit::Home(2)));
        app.home_selected = 2;
        let right = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        let left = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
        app.key(right).unwrap();
        assert_eq!(app.home_selected, 6);
        app.key(right).unwrap();
        assert_eq!(app.home_selected, 6);
        app.key(left).unwrap();
        assert_eq!(app.home_selected, 2);
        app.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
            .unwrap();
        app.key(right).unwrap();
        assert_eq!(app.home_selected, 7);
        app.key(left).unwrap();
        assert_eq!(app.home_selected, 3);

        app.hits = (0..8)
            .map(|index| {
                (
                    ratatui::layout::Rect::new(3, 5 + index as u16, 54, 1),
                    Hit::Home(index),
                )
            })
            .collect();
        app.hits
            .push((ratatui::layout::Rect::new(3, 20, 54, 2), Hit::Home(3)));
        app.key(right).unwrap();
        app.key(left).unwrap();
        assert_eq!(app.home_selected, 3);
        assert_eq!(app.page, Page::Home);
        assert!(app.dialog.is_none());
        assert!(!app.root.join("preferences.toml").exists());
    }

    #[test]
    fn appearance_keyboard_choices_save_live_without_losing_drafts_or_changing_connections() {
        let (_directory, mut app) = setup();
        app.composer.replace("Keep this unsent request");
        app.page = Page::Chat;
        let original_config = toml::to_string(&app.config).unwrap();
        let original_project = app.preferences.project.clone();
        let original_context = app.preferences.context_tokens;

        palette(&mut app, "Appearance");
        assert!(
            matches!(&app.dialog, Some(Dialog::Menu { title, selected: 0, .. }) if title == "Appearance")
        );
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        app.key(enter).unwrap();
        app.key(down).unwrap();
        app.key(down).unwrap();
        app.key(enter).unwrap();
        assert_eq!(
            app.preferences.appearance.theme,
            crate::config::ThemePreset::Aurora
        );
        assert!(
            matches!(&app.dialog, Some(Dialog::Menu { title, selected: 2, .. }) if title == "Color theme")
        );
        assert_eq!(
            Preferences::load(&app.root).unwrap().appearance.theme,
            crate::config::ThemePreset::Aurora
        );

        app.setting_action(17).unwrap();
        app.key(down).unwrap();
        app.key(enter).unwrap();
        for _ in 0..3 {
            app.key(down).unwrap();
        }
        app.key(enter).unwrap();
        assert_eq!(
            app.preferences.appearance.layout,
            crate::config::LayoutPreset::Focus
        );
        app.menu_action(MenuAction::Decorations).unwrap();
        let loaded = Preferences::load(&app.root).unwrap();
        assert!(!loaded.appearance.decorations);
        assert_eq!(loaded.appearance.layout, crate::config::LayoutPreset::Focus);
        assert_eq!(loaded.context_tokens, original_context);
        assert_eq!(loaded.project, original_project);
        assert_eq!(app.page, Page::Chat);
        assert_eq!(app.composer.text, "Keep this unsent request");
        assert_eq!(toml::to_string(&app.config).unwrap(), original_config);

        app.menu_action(MenuAction::ResetAppearance).unwrap();
        assert_eq!(
            Preferences::load(&app.root).unwrap().appearance,
            crate::config::Appearance::default()
        );
        assert_eq!(app.composer.text, "Keep this unsent request");
    }

    #[test]
    fn invalid_accent_and_failed_appearance_save_preserve_input_and_current_preferences() {
        let (_directory, mut app) = setup();
        app.menu_action(MenuAction::Theme(crate::config::ThemePreset::Ember))
            .unwrap();
        let stored = std::fs::read(app.root.join("preferences.toml")).unwrap();
        app.menu_action(MenuAction::AccentColor).unwrap();
        app.paste("#nope");
        let error = app
            .key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .unwrap_err();
        assert!(error.to_string().contains("six hexadecimal digits"));
        assert!(
            matches!(&app.dialog, Some(Dialog::Input { editor, action: InputAction::AccentColor, .. }) if editor.text == "#nope")
        );
        assert_eq!(
            std::fs::read(app.root.join("preferences.toml")).unwrap(),
            stored
        );
        app.submit_input(InputAction::AccentColor, "#7aa2f7".into())
            .unwrap();
        assert_eq!(
            app.preferences.appearance.accent.as_deref(),
            Some("#7AA2F7")
        );
        let before = app.preferences.appearance.clone();
        std::fs::rename(
            app.root.join("preferences.toml"),
            app.root.join("preserved.toml"),
        )
        .unwrap();
        std::fs::create_dir(app.root.join("preferences.toml")).unwrap();
        assert!(
            app.menu_action(MenuAction::Theme(crate::config::ThemePreset::Daylight))
                .is_err()
        );
        assert_eq!(app.preferences.appearance, before);
        assert!(matches!(&app.dialog, Some(Dialog::Menu { title, .. }) if title == "Color theme"));
    }

    #[tokio::test]
    async fn global_palette_opens_setup_and_recovery_without_dead_form_actions() {
        let (_directory, mut app) = setup();
        palette(&mut app, "Add connection");
        assert!(
            matches!(&app.dialog, Some(Dialog::Menu { title, .. }) if title == "Where will your model run?")
        );
        palette(&mut app, "Open model repository");
        assert_eq!(app.page, Page::Models);
        assert_eq!(app.model_tab, 2);
        assert!(matches!(
            &app.dialog,
            Some(Dialog::Input {
                action: InputAction::DirectRepo,
                ..
            })
        ));
        palette(&mut app, "Storage and recovery");
        assert!(
            matches!(&app.dialog, Some(Dialog::Menu { title, .. }) if title == "Storage and recovery")
        );
        palette(&mut app, "Search conversations");
        assert_eq!(app.page, Page::Sessions);
        assert!(app.search_focus);
        assert!(super::super::actions::search("manual-model").is_empty());
        assert!(super::super::actions::search("test-form").is_empty());
        app.dialog = None;
        app.search_focus = false;
        app.key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL))
            .unwrap();
        assert!(app.dialog.is_none());
        app.menu_action(MenuAction::Preset(1)).unwrap();
        app.key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL))
            .unwrap();
        assert!(matches!(
            &app.dialog,
            Some(Dialog::Input {
                action: InputAction::ManualModel(_),
                ..
            })
        ));
    }

    #[test]
    fn mouse_wheel_scrolls_settings_without_changing_or_wrapping_them() {
        let (_directory, mut app) = setup();
        app.page = Page::Settings;
        let original_context = app.preferences.context_tokens;
        for _ in 0..10 {
            app.mouse(MouseEventKind::ScrollDown, 40, 15).unwrap();
        }
        assert_eq!(app.settings_selected, 17);
        assert_eq!(app.preferences.context_tokens, original_context);
        assert!(app.dialog.is_none());
        assert!(!app.root.join("preferences.toml").exists());
        for _ in 0..10 {
            app.mouse(MouseEventKind::ScrollUp, 40, 15).unwrap();
        }
        assert_eq!(app.settings_selected, 0);
        app.key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT))
            .unwrap();
        assert!(app.nav_focus);
        app.key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT))
            .unwrap();
        assert!(!app.nav_focus);
    }

    #[test]
    fn mouse_wheel_scrolls_long_browser_and_menu_but_leaves_form_text_alone() {
        let (directory, mut app) = setup();
        app.dialog = Some(Dialog::Browser {
            kind: BrowserKind::Project,
            path: directory.path().into(),
            entries: (0..50)
                .map(|n| directory.path().join(format!("folder-{n:02}")))
                .collect(),
            selected: 0,
        });
        for _ in 0..20 {
            app.mouse(MouseEventKind::ScrollDown, 40, 15).unwrap();
        }
        assert!(matches!(
            &app.dialog,
            Some(Dialog::Browser { selected: 49, .. })
        ));
        app.connection_wizard();
        app.mouse(MouseEventKind::ScrollDown, 40, 15).unwrap();
        assert!(matches!(
            &app.dialog,
            Some(Dialog::Menu { selected: 3, .. })
        ));
        app.mouse(MouseEventKind::ScrollDown, 40, 15).unwrap();
        assert!(matches!(
            &app.dialog,
            Some(Dialog::Menu { selected: 4, .. })
        ));
        app.mouse(MouseEventKind::ScrollDown, 40, 15).unwrap();
        assert!(matches!(
            &app.dialog,
            Some(Dialog::Menu { selected: 4, .. })
        ));
        app.menu_action(MenuAction::Preset(1)).unwrap();
        app.mouse(MouseEventKind::ScrollDown, 40, 15).unwrap();
        assert!(matches!(&app.dialog, Some(Dialog::Connection(form)) if form.focus == 1));
    }
}
