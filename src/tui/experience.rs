//! Resumable beginner steps and explicit draft/requirement recovery.
use super::{app::*, drafts::Submission, input::Editor};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
impl App {
    pub fn remember_draft(&mut self) {
        self.drafts.remember(
            &self.preferences.project,
            self.session.as_ref().map(|s| s.id.as_str()),
            &self.composer.text,
            self.pending_prompt.as_ref(),
            self.composer_message_id.as_deref(),
        );
    }
    pub fn draft_menu(&mut self) {
        self.remember_draft();
        let mut items = Vec::new();
        if let Some(pending) = &self.pending_prompt {
            items.push((
                "Restore interrupted submission".into(),
                format!(
                    "{} · Nothing will be sent until you press Send",
                    pending.text.chars().take(80).collect::<String>()
                ),
                "draft-pending".into(),
                json!({}),
            ));
            items.push((
                "Discard interrupted submission".into(),
                "Explicitly remove the submitted draft; keep the current composer".into(),
                "draft-discard-pending-review".into(),
                json!({}),
            ));
        }
        for (id, draft) in &self.drafts.entries {
            if id == &self.drafts.active || draft.text.is_empty() && draft.pending.is_none() {
                continue;
            }
            let preview = if draft.text.is_empty() {
                draft
                    .pending
                    .as_ref()
                    .map(|p| p.text.as_str())
                    .unwrap_or("")
            } else {
                &draft.text
            };
            items.push((
                format!("Restore: {}", preview.chars().take(60).collect::<String>()),
                draft.project.display().to_string(),
                "draft-restore".into(),
                json!({"id":id}),
            ));
            items.push((
                format!(
                    "Discard retained draft {}",
                    id.chars().take(8).collect::<String>()
                ),
                "Review before permanently removing this unsent text".into(),
                "draft-discard-review".into(),
                json!({"id":id}),
            ));
        }
        items.push((
            "Save drafts now".into(),
            "Flush text to local storage and report the result".into(),
            "draft-save".into(),
            json!({}),
        ));
        self.manager_menu("Saved drafts", "Drafts save every 250 ms while storage is responsive. Orderly exit flushes them. An abrupt stop can lose edits since the last save; interrupted submissions are never automatically resent.", items);
    }
    pub fn draft_action(&mut self, action: &str, state: Value) -> Result<()> {
        ensure!(
            !self.busy && !self.connecting,
            "Stop the current task before restoring or discarding drafts"
        );
        match action {
            "draft-save" => {
                self.remember_draft();
                self.flush_drafts = true;
                self.dialog = None;
            }
            "draft-pending" => {
                let pending = self
                    .pending_prompt
                    .take()
                    .context("No interrupted submission")?;
                self.remember_draft();
                self.drafts.current_mut().archived = true;
                self.drafts.fresh(&self.preferences.project);
                self.composer = Editor::new(&pending.text);
                self.composer_message_id = Some(pending.id);
                self.dialog = None;
                self.set_page(Page::Chat);
                self.notify("Submission restored. Your next-message draft is retained in Saved drafts. Send explicitly when ready.");
            }
            "draft-discard-pending-review" => self.manager_confirm(
                "Discard this interrupted submission?",
                self.pending_prompt
                    .as_ref()
                    .context("No interrupted submission")?
                    .text
                    .clone(),
                "draft-discard-pending",
                state,
            ),
            "draft-discard-pending" => {
                self.pending_prompt = None;
                self.remember_draft();
                self.dialog = None;
                self.notify("Interrupted submission discarded. Your next draft is unchanged.");
            }
            "draft-restore" => {
                let id = state["id"].as_str().context("Draft identifier")?;
                let draft = self
                    .drafts
                    .entries
                    .get(id)
                    .context("Saved draft missing")?
                    .clone();
                self.remember_draft();
                self.drafts.current_mut().archived = true;
                self.choose_project(&draft.project)?;
                self.drafts.fresh(&draft.project);
                self.composer = Editor::new(draft.text);
                self.composer_message_id = draft.composer_message_id;
                self.pending_prompt = draft.pending;
                self.send_pending_on_ready = false;
                self.dialog = None;
                self.set_page(Page::Chat);
                self.remember_draft();
                self.notify("Draft restored into this project. Nothing was sent; use Saved drafts for any interrupted submission.");
            }
            "draft-discard-review" => {
                let id = state["id"].as_str().context("Draft identifier")?;
                let draft = self.drafts.entries.get(id).context("Saved draft missing")?;
                self.manager_confirm(
                    "Permanently discard this retained draft?",
                    format!(
                        "{}\n\n{}\n{}",
                        draft.project.display(),
                        draft.text,
                        draft
                            .pending
                            .as_ref()
                            .map(|p| p.text.as_str())
                            .unwrap_or("")
                    ),
                    "draft-discard",
                    state,
                );
            }
            "draft-discard" => {
                let id = state["id"].as_str().context("Draft identifier")?;
                ensure!(
                    id != self.drafts.active,
                    "The current draft cannot be discarded here"
                );
                self.drafts.entries.remove(id);
                self.dialog = None;
                self.notify("Retained draft explicitly discarded.");
            }
            _ => anyhow::bail!("Unknown draft action"),
        }
        Ok(())
    }
    pub fn manual_model(&mut self) -> Result<()> {
        let Some(Dialog::Connection(form)) = &self.dialog else {
            anyhow::bail!(
                "Open Connections → Add or Edit first, then use Enter model ID or Ctrl+O"
            );
        };
        let draft = self.draft(form)?;
        let initial = if draft.profile.model == "choose-next" {
            ""
        } else {
            &draft.profile.model
        };
        self.dialog = Some(Dialog::Input {
            title: "Enter a model identifier".into(),
            hint: "Use the exact name from your server. This saves without checking its inventory."
                .into(),
            editor: Editor::new(initial),
            action: InputAction::ManualModel(draft),
            multiline: false,
        });
        Ok(())
    }
    pub fn next_step(&self) -> (&'static str, &'static str, &'static str) {
        let Some((_, profile)) = self.current_profile() else {
            return (
                "Choose where your model runs",
                "Connect your server or import a model file.",
                "connect",
            );
        };
        if self.preferences.recent_projects.is_empty() {
            return (
                "Choose the project or try a practice project",
                "Pick the files you want to work on. A practice project provides real checks and undo.",
                "project",
            );
        }
        if !self.engine_ready() {
            return (
                "Install the agent engine",
                "This gives the selected model its tools.",
                "install-engine",
            );
        }
        if profile.local_model.is_some() && !self.runtime_ready() {
            return (
                "Install or choose a model runtime",
                "CPU works without a GPU. Settings can select your own compatible executable.",
                "runtime",
            );
        }
        if self.session.is_none() && self.messages.is_empty() {
            return (
                "Send your first request",
                "Ask a question or describe your goal. You can prepare project checks when you need them.",
                "page-chat",
            );
        }
        if self.task_view.checks.is_empty() {
            return (
                "Optional: prepare a project check",
                "For code changes, choose a command that should prove the result. You can keep chatting without one.",
                "task-configure",
            );
        }
        (
            "Inspect the result and try undo",
            "Review actual file changes and current checks in Task progress.",
            "task",
        )
    }
    pub fn understanding(&mut self) -> Result<()> {
        self.launch_project("Reading current requirements", |p, task| {
            let mut body = format!("Original goal\n{}\n\nActive user requirements\n",p.task(&task)?.goal);
            for request in p.active_requests(&task)? {
                body.push_str(&format!("\nRequest {} · {}\n{}\n",request.seq,request.source,request.body));
            }
            let retired: Vec<_> = p.request_history(&task)?.into_iter().filter(|r|r.superseded_by.is_some()).collect();
            if !retired.is_empty() { body.push_str("\nRetained earlier requirements (explicitly replaced)\n"); }
            for request in retired { body.push_str(&format!("\nRequest {} → {} · {}\n{}\n",request.seq,request.superseded_by.unwrap(),request.superseded_reason.unwrap_or_default(),request.body)); }
            body.push_str(&format!("\nPinned requirements\n{}\n\nUser corrections keep the exact earlier text in history. Use the Correct a requirement action to explicitly replace a request; model summaries cannot retire it.",serde_json::to_string_pretty(&p.pinned()?)?));
            Ok(JobResult::Notice("What Alt currently understands".into(),body))
        })
    }
    pub fn correct_requirement(&mut self) -> Result<()> {
        ensure!(
            !self.busy && !self.connecting,
            "Stop the current task before correcting requirements"
        );
        self.launch_project("Reading active user requirements", |p, task| {
            let items = p.active_requests(&task)?.into_iter().map(|r| (format!("Request {}: {}",r.seq,r.body.chars().take(80).collect::<String>()), "Replace this exact request; keep the original in history".into(),MenuAction::Manager {action:"requirement-edit".into(),state:json!({"task":task,"seq":r.seq,"original":r.body})})).collect();
            Ok(JobResult::Dialog(Dialog::Menu {title:"Choose a requirement to correct".into(),description:"Only your explicit correction retires a user request. Editing a summary cannot remove it.".into(),items,selected:0}))
        })
    }
    pub fn requirement_action(&mut self, action: &str, mut state: Value) -> Result<()> {
        ensure!(
            !self.busy && !self.connecting,
            "Stop the current task before correcting requirements"
        );
        match action {
            "requirement-edit" => self.dialog = Some(Dialog::Input {title:"Replace this user requirement".into(),hint:"Ctrl+S continues. The exact earlier request remains in history; unrelated requirements stay active.".into(),editor:Editor::new(state["original"].as_str().context("Original requirement")?),action:InputAction::Manager{action:"requirement-reason".into(),state},multiline:true}),
            "requirement-reason" => { let text=state["value"].as_str().context("Replacement")?;ensure!(!text.trim().is_empty(),"Write the replacement requirement");state["replacement"]=json!(text); self.manager_input("Why is this requirement changing?","Describe your explicit correction for the saved history","requirement-review",state,""); },
            "requirement-review" => { let reason=state["value"].as_str().context("Correction reason")?.to_owned();ensure!(!reason.trim().is_empty(),"Explain this correction");state["reason"]=json!(reason);self.manager_confirm("Replace this requirement?",format!("Earlier request:\n{}\n\nReplacement:\n{}\n\nReason:\n{}\n\nNo model request is sent. Unrelated active requirements stay in place.",state["original"].as_str().unwrap_or(""),state["replacement"].as_str().unwrap_or(""),reason),"requirement-save",state); },
            "requirement-save" => {
                let task=state["task"].as_str().context("Task")?.to_owned();let seq=state["seq"].as_i64().context("Request")?;let replacement=state["replacement"].as_str().context("Replacement")?.to_owned();let reason=state["reason"].as_str().context("Reason")?.to_owned();
                self.launch_project("Saving your explicit correction",move|p,current|{ensure!(current==task,"The selected task changed; review this correction again");p.supersede_request(&task,seq,&replacement,&reason)?;Ok(JobResult::Saved("Requirement corrected. What Alt currently understands shows the active request and retained history.".into()))})?;
            }
            _ => anyhow::bail!("Unknown requirement action"),
        }
        Ok(())
    }
    pub fn recovered_files(&mut self) -> Result<()> {
        self.launch_project("Reading recovered file versions", |p,_| {
            let rows = p.displaced_versions()?;
            if rows.is_empty() { return Ok(JobResult::Notice("Recovered file versions".into(),"No displaced file versions are recorded for this project. If an external edit conflicts with apply or undo, recoverable versions appear here.".into())); }
            let items = rows.into_iter().map(|row| (format!("{} · {}",row["path"].as_str().unwrap_or("file"),row["operation"].as_str().unwrap_or("conflict")),format!("Version {} · preview before restoring",row["sha256"].as_str().unwrap_or("").chars().take(12).collect::<String>()),MenuAction::Manager {action:"recovered-preview".into(),state:row})).collect();
            Ok(JobResult::Dialog(Dialog::Menu { title:"Recovered file versions".into(),description:"Choose a displaced version to preview it. Restoring is explicit and preserves a tracked undo checkpoint.".into(),items,selected:0 }))
        })
    }
    pub fn model_fit_action(&mut self, action: &str, state: Value) -> Result<()> {
        match action {
            "model-fit" => {
                let root = self.root.clone();
                let prefs = self.preferences.clone();
                let (name, profile) = self.current_profile().context("Choose a model first")?;
                let name = name.to_owned();
                let profile = profile.clone();
                self.launch_job("Estimating the selected model's memory",move|cancel,_|async move {
                    let report=tokio::select!{r=crate::hardware::model_fit(&root,&prefs,&profile)=>r?,_=crate::models::cancelled(&cancel)=>anyhow::bail!("Memory estimate cancelled")};
                    Ok(JobResult::Manager("model-fit-result".into(),json!({"profile":name,"identity":profile,"runtime_identity":prefs.runtime,"report":report})))
                })?;
            }
            "model-fit-result" => {
                let r = &state["report"];
                let Some(proposals) = r["proposals"].as_array() else {
                    self.dialog = Some(Dialog::Notice {
                        title: "Selected model memory is unmeasured".into(),
                        body: format!(
                            "{}\n\n{}\n\nSettings → Benchmark selected model records actual timings and memory. No model or setting was changed.",
                            r["model"].as_str().unwrap_or("Selected model"),
                            r["reason"]
                                .as_str()
                                .unwrap_or("Model metadata is unavailable")
                        ),
                        scroll: 0,
                    });
                    return Ok(());
                };
                let mut items = Vec::new();
                for proposal in proposals {
                    let size = |field: &str| {
                        proposal[field]
                            .as_u64()
                            .map(crate::models::human_bytes)
                            .unwrap_or_else(|| "unknown".into())
                    };
                    let mut chosen = state.clone();
                    chosen["proposal"] = proposal.clone();
                    items.push((
                        format!(
                            "{}K context · {} GPU layers",
                            proposal["context"].as_u64().unwrap_or(0) / 1024,
                            proposal["gpu_layers"]
                        ),
                        format!(
                            "Estimated RAM {} · VRAM {} · review before applying",
                            size("estimated_ram_bytes"),
                            size("estimated_vram_bytes")
                        ),
                        "model-fit-review".into(),
                        chosen,
                    ));
                }
                self.manager_menu("Estimated choices for your selected model","Estimates are not measured fit or GPU compatibility. Review a choice, then explicitly benchmark it. Existing context/output allowances stay unchanged until confirmation.",items);
            }
            "model-fit-review" => {
                let p = &state["proposal"];
                let bytes = |field: &str| {
                    p[field]
                        .as_u64()
                        .map(crate::models::human_bytes)
                        .unwrap_or_else(|| "unknown".into())
                };
                self.manager_confirm("Apply this estimated model configuration?",format!("Selected model: {}\nContext: {} tokens\nGPU layers: {}\nBatch: {}\nK/V cache: {} / {}\n\nEstimated RAM: {}\nEstimated VRAM: {}\n\n{}\n\nThis saves the reviewed configuration for the next connection. It does not start inference, change weights, raise output allowance or establish hardware compatibility. Use Benchmark selected model to measure it.",state["identity"]["model"].as_str().unwrap_or(""),p["context"],p["gpu_layers"],p["batch"],p["cache_k"].as_str().unwrap_or(""),p["cache_v"].as_str().unwrap_or(""),bytes("estimated_ram_bytes"),bytes("estimated_vram_bytes"),p["scope"].as_str().unwrap_or("Estimate only")),"model-fit-apply",state);
            }
            "model-fit-apply" => {
                ensure!(
                    !self.busy && !self.connecting,
                    "Stop the current task before changing allocation"
                );
                let name = state["profile"].as_str().context("Profile")?;
                let before = self
                    .config
                    .profiles
                    .get(name)
                    .context("Profile changed; estimate again")?;
                ensure!(
                    serde_json::to_value(before)? == state["identity"],
                    "Selected model settings changed; estimate and review again"
                );
                ensure!(
                    serde_json::to_value(&self.preferences.runtime)? == state["runtime_identity"],
                    "Runtime settings changed; estimate and review again"
                );
                let p = &state["proposal"];
                let mut profile = before.clone();
                profile.context_tokens = u32::try_from(p["context"].as_u64().context("Context")?)?;
                profile.validate()?;
                let mut prefs = self.preferences.clone();
                prefs.context_tokens = profile.context_tokens;
                prefs.runtime.gpu_layers =
                    i32::try_from(p["gpu_layers"].as_i64().context("GPU layers")?)?;
                prefs.runtime.validate()?;
                let mut config = self.config.clone();
                config.profiles.insert(name.into(), profile);
                config.save(&self.root)?;
                if let Err(error) = prefs.save(&self.root) {
                    if let Err(rollback) = self.config.save(&self.root) {
                        anyhow::bail!(
                            "Runtime settings could not be saved: {error:#}. Restoring the previous profile also failed: {rollback:#}. Reopen Settings and inspect context and GPU layers before reconnecting"
                        );
                    }
                    return Err(error).context(
                        "Runtime settings could not be saved; the previous profile was restored",
                    );
                }
                self.config = config;
                self.preferences = prefs;
                self.notify("Reviewed memory configuration saved. Reconnect explicitly, then benchmark it; fit and speed remain unmeasured.");
            }
            _ => anyhow::bail!("Unknown model fit action"),
        }
        Ok(())
    }
    pub fn submitted(&mut self) -> Submission {
        {
            let mut submission = Submission::new(self.composer.text.clone());
            if let Some(id) = self.composer_message_id.take() {
                submission.id = id;
            }
            submission
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Profile, Provider};

    #[test]
    fn first_request_needs_no_project_check_or_programming_command() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = App::load(
            directory.path().join("state"),
            std::env::current_exe().unwrap(),
            None,
        )
        .unwrap();
        assert_eq!(app.next_step().2, "connect");
        app.config.profiles.insert(
            "test".into(),
            Profile {
                provider: Provider::Openai,
                endpoint: "http://127.0.0.1:8080/v1".into(),
                model: "fixture".into(),
                context_tokens: 4096,
                max_turns: 12,
                uncensored: false,
                api_key_env: None,
                local_model: None,
                inference: None,
            },
        );
        app.config.default_profile = "test".into();
        assert_eq!(app.next_step().2, "project");
        app.preferences
            .recent_projects
            .push(directory.path().into());
        assert_eq!(app.next_step().2, "page-chat");
        assert!(app.task_view.checks.is_empty());
        app.add_message("You", "Please explain this folder.");
        assert!(app.next_step().0.starts_with("Optional:"));
        assert!(app.next_step().1.contains("keep chatting"));
    }
}
