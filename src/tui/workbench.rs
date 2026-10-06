use super::{app::*, input::Editor};
use crate::{
    jobs,
    project::{Policy, Project},
    project_services::Requirement,
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::sync::mpsc;
#[derive(Default)]
pub struct Workbench {
    pub files: Vec<String>,
    pub file_selected: usize,
    pub filter: String,
    pub jobs: Vec<jobs::Record>,
    pub job_selected: usize,
    pub screen: String,
    pub attached: bool,
    pub terminal: Option<TerminalLink>,
    pub dimensions: (u16, u16),
    pub context: String,
}
pub struct TerminalLink {
    pub tx: mpsc::Sender<Value>,
    pub rx: mpsc::Receiver<Result<Value, String>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for TerminalLink {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl App {
    pub fn workbench_action(&mut self, action: &str) -> Result<()> {
        match action {
            "files-refresh" => self.launch_project("Reading project files", |p, _| {
                Ok(JobResult::ProjectFiles(p.browse()?))
            })?,
            "files-search" => {
                self.dialog = Some(Dialog::Input {
                    title: "Filter project files".into(),
                    hint: "Match part of a file or folder name. Empty shows all files.".into(),
                    editor: Editor::new(&self.workbench.filter),
                    action: InputAction::FileFilter,
                    multiline: false,
                })
            }
            "file-view" => {
                let path = self.selected_file()?;
                self.show_file_page(path, 1)?;
            }
            "file-line" | "file-find" => {
                let path = self.selected_file()?;
                let find = action == "file-find";
                self.dialog = Some(Dialog::Input {
                    title: if find {
                        format!("Find text in {path}")
                    } else {
                        format!("Go to line in {path}")
                    },
                    hint: if find {
                        "Case-insensitive text search with matching line numbers.".into()
                    } else {
                        "Enter a line number. Shows up to 300 lines from there.".into()
                    },
                    editor: Editor::default(),
                    action: if find {
                        InputAction::FileText(path)
                    } else {
                        InputAction::FileLine(path)
                    },
                    multiline: false,
                });
            }
            "file-edit" => {
                ensure!(
                    self.preferences.access_policy != Policy::ReviewOnly,
                    "Choose Guided changes or Full access to edit files"
                );
                let path = self.selected_file()?;
                self.edit_project_file(path, false)?;
            }
            "file-new" => {
                ensure!(
                    self.preferences.access_policy != Policy::ReviewOnly,
                    "Choose Guided changes or Full access to create files"
                );
                self.dialog = Some(Dialog::Input {
                    title: "Create a file".into(),
                    hint: "Enter a relative path in an existing project folder.".into(),
                    editor: Editor::default(),
                    action: InputAction::NewFile,
                    multiline: false,
                });
            }
            "all-diffs" => self.launch_project("Reading change history", |p, _| {
                let body = p
                    .changes(None)?
                    .iter()
                    .map(|c| format!("{} · {} · {}\n{}\n", c.path, c.status, c.reason, c.diff()))
                    .collect::<String>();
                Ok(JobResult::Notice(
                    "Project change history".into(),
                    if body.is_empty() {
                        "No changes saved yet.".into()
                    } else {
                        body
                    },
                ))
            })?,
            "jobs-refresh" => {
                let root = self.root.clone();
                let cwd = self.preferences.project.clone();
                self.launch_job("Reading terminal jobs", move |_, _| async move {
                    Ok(JobResult::Jobs(
                        tokio::task::spawn_blocking(move || -> Result<_> {
                            Ok(jobs::list(&root)?
                                .into_iter()
                                .filter(|r| r.spec.cwd == cwd)
                                .collect())
                        })
                        .await??,
                    ))
                })?;
            }
            "job-new" => {
                ensure!(
                    self.preferences.access_policy == Policy::Trusted,
                    "Choose Full access in Settings to start a terminal job"
                );
                self.dialog=Some(Dialog::Input{title:"Start an interactive command".into(),hint:"Examples: bash, npm run dev, cargo build. Runs in this project with Full access. Next choose its lifetime.".into(),editor:Editor::default(),action:InputAction::JobCommand,multiline:false});
            }
            "job-attach" => {
                ensure!(
                    self.preferences.access_policy == Policy::Trusted,
                    "Choose Full access before sending terminal input"
                );
                let id = self.selected_job()?.id.clone();
                self.connect_terminal(id)?;
                self.workbench.attached = true;
                self.notify(
                    "Terminal input is active. Ctrl+] detaches; Ctrl+C goes to the program.",
                );
            }
            "job-detach" => {
                self.workbench.attached = false;
                self.notify("Detached from input. The job continues according to its lifetime.");
            }
            "job-stop" | "job-restart" => {
                let id = self.selected_job()?.id.clone();
                let root = self.root.clone();
                let restart = action == "job-restart";
                if restart {
                    ensure!(
                        self.preferences.access_policy == Policy::Trusted,
                        "Choose Full access before restarting a command"
                    );
                }
                self.workbench.terminal = None;
                self.workbench.attached = false;
                self.launch_job(
                    if restart {
                        "Restarting job"
                    } else {
                        "Stopping job"
                    },
                    move |_, _| async move {
                        let r = if restart {
                            jobs::restart(&root, &id).await?
                        } else {
                            jobs::stop(&root, &id).await?
                        };
                        Ok(JobResult::ManagedJob(r))
                    },
                )?;
            }
            "job-logs" => {
                let record = self.selected_job()?.clone();
                let root = self.root.clone();
                self.launch_job("Reading job output", move |_, _| async move {
                    let body = tokio::task::spawn_blocking(move || jobs::output(&root, &record.id))
                        .await??;
                    Ok(JobResult::Notice(
                        format!("{} · {}", record.spec.name, record.status),
                        body,
                    ))
                })?;
            }
            "job-health" => {
                let id = self.selected_job()?.id.clone();
                self.dialog=Some(Dialog::Input { title:"Check this service's health".into(),hint:"URL to request; HTTP 200–299 indicates a responding service, not full behavioral verification.".into(),editor:Editor::new("http://127.0.0.1:3000/"),action:InputAction::JobHealth(id),multiline:false });
            }
            "context-refresh" => {
                let brief = self.brief.clone();
                self.launch_project("Reading project context", move |p, task| Ok(JobResult::Context(format!("User project brief:\n{brief}\n\nPinned requirements:\n{}\n\nLast assembled model memory:\n{}", serde_json::to_string_pretty(&p.pinned()?)?, serde_json::to_string_pretty(&p.context_view(&task)?)?))))?;
            }
            "pin-requirement" => self.dialog = Some(Dialog::Input {
                title: "Pin an important requirement".into(),
                hint:
                    "A durable user requirement included in future bounded contexts. Ctrl+S saves."
                        .into(),
                editor: Editor::default(),
                action: InputAction::PinRequirement,
                multiline: true,
            }),
            "unpin-requirement" => {
                self.launch_project("Reading pinned requirements", |p, _| {
                    Ok(JobResult::Dialog(Dialog::Menu {
                        title: "Remove a pinned requirement".into(),
                        description: "The original conversation remains saved.".into(),
                        items: p
                            .pinned()?
                            .iter()
                            .map(|v| {
                                (
                                    v["requirement"].as_str().unwrap_or("").to_string(),
                                    v["id"].as_str().unwrap_or("").to_string(),
                                    MenuAction::Unpin(v["id"].as_str().unwrap_or("").to_string()),
                                )
                            })
                            .collect(),
                        selected: 0,
                    }))
                })?;
            }
            "verification-plan" => {
                self.launch_project("Reading verification plan", |p, _| {
                Ok(JobResult::Dialog(Dialog::Menu{title:"Choose a required check".into(),description:"Each required check must pass on current files. Select a check to toggle whether it is required.".into(),items:p.checks()?.iter().map(|c|(c.name.clone(),if p.requirements().unwrap_or_default().iter().any(|r|r.check_name==c.name){"Required · select to make optional".into()}else{"Optional · select to require".into()},MenuAction::RequireCheck(c.name.clone()))).collect(),selected:0}))
                })?;
            }
            "verification-status" => {
                self.launch_project("Reading verification evidence", |p, _| {
                    let v = p.verification()?;
                    let mut body: String = if v.complete {
                        format!("{}\n", v.scope)
                    } else {
                        "Required checks are not complete.\n".into()
                    };
                    for r in v.requirements {
                        body.push_str(&format!(
                            "\n{}: {}\n{}\nEvidence: {}\n",
                            r.requirement.name,
                            r.status,
                            format_args!("{}\nCoverage: {}", r.requirement.description, r.coverage),
                            r.evidence.as_deref().unwrap_or("none")
                        ));
                    }
                    Ok(JobResult::Dialog(Dialog::Notice {
                        title: "Task verification plan".into(),
                        body,
                        scroll: 0,
                    }))
                })?;
            }
            "verification-run" => {
                let root = self.root.clone();
                let cwd = self.preferences.project.clone();
                let preferred = self.session.as_ref().map(|s| s.id.clone());
                let policy = self.preferences.access_policy;
                self.launch_job("Running required checks", move |cancel, _| async move {
                    let (task, specs) = crate::project_worker::run(
                        root.clone(),
                        cwd.clone(),
                        cancel.clone(),
                        move |p| {
                            Ok((
                                crate::project_worker::task(p, preferred)?,
                                p.requirements()?,
                            ))
                        },
                    )
                    .await?;
                    ensure!(!specs.is_empty(), "Choose required checks first");
                    for r in specs {
                        crate::sandbox::run(
                            root.clone(),
                            cwd.clone(),
                            task.clone(),
                            r.check_name,
                            policy,
                            cancel.clone(),
                        )
                        .await?;
                        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                            break;
                        }
                    }
                    Ok(JobResult::Notice(
                        "Required check results".into(),
                        crate::project_worker::run(root, cwd, cancel, |p| {
                            Ok(serde_json::to_string_pretty(&p.verification()?)?)
                        })
                        .await?,
                    ))
                })?;
            }
            _ => {}
        }
        Ok(())
    }
    pub fn filtered_files(&self) -> Vec<&String> {
        self.workbench
            .files
            .iter()
            .filter(|p| {
                p.to_lowercase()
                    .contains(&self.workbench.filter.to_lowercase())
            })
            .collect()
    }
    pub fn show_file_page(&mut self, path: String, start: usize) -> Result<()> {
        self.launch_project("Reading file", move |p, task| {
        let read = p.read(&task, &path, start, 300)?;
        let total = read["total_lines"].as_u64().unwrap_or(0) as usize;
        ensure!(start <= total.max(1), "This file has {total} lines");
        Ok(JobResult::Dialog(Dialog::Notice {
            title: path,
            body: format!(
                "{total} lines · from {start}, up to 300 lines / 24,000 characters\nClose, then G to go to a line or S to find text.\n\n{}",
                read["text"].as_str().unwrap_or("")
            ),
            scroll: 0,
        }))
        })
    }
    fn selected_file(&self) -> Result<String> {
        Ok((*self
            .filtered_files()
            .get(self.workbench.file_selected)
            .context("Choose a project file")?)
        .clone())
    }
    fn selected_job(&self) -> Result<&jobs::Record> {
        self.workbench
            .jobs
            .get(self.workbench.job_selected)
            .context("Start or select a job first")
    }
    pub fn edit_project_file(&mut self, path: String, new: bool) -> Result<()> {
        self.launch_project("Opening file editor", move |p, task| {
        Project::validate_path(&path)?;
        let original = if new {
            ensure!(
                !p.root.join(&path).exists(),
                "That file already exists; select Edit"
            );
            String::new()
        } else {
            p.read(&task, &path, 1, 1)?;
            let body = p.dir.read_to_string(&path)?;
            ensure!(
                body.len() <= 64 * 1024,
                "The built-in editor supports 64 KiB; use a Full access terminal editor for larger files"
            );
            body
        };
        Ok(JobResult::Dialog(Dialog::Input {
            title: format!("Edit {path}"),
            hint: "Ctrl+S previews the change before saving; Esc leaves the file unchanged.".into(),
            editor: Editor::new(&original),
            action: InputAction::EditFile {
                path,
                original,
                task,
                new,
            },
            multiline: true,
        }))
        })
    }
    pub fn start_terminal_job(
        &mut self,
        command: String,
        keep: bool,
        timeout_secs: u64,
    ) -> Result<()> {
        let root = self.root.clone();
        let cwd = self.preferences.project.clone();
        self.launch_job("Starting terminal job", move |_, _| async move {
            Ok(JobResult::ManagedJob(
                jobs::start(
                    &root,
                    jobs::Spec {
                        name: crate::project::bounded(&command, 100),
                        command,
                        cwd,
                        keep,
                        timeout_secs,
                        rows: 24,
                        cols: 100,
                    },
                )
                .await?,
            ))
        })
    }
    pub fn connect_terminal(&mut self, id: String) -> Result<()> {
        let root = self.root.clone();
        let (tx, mut commands) = mpsc::channel::<Value>(64);
        let (updates, rx) = mpsc::channel(8);
        let task = tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_millis(100));
            loop {
                let value = tokio::select! {_=tick.tick()=>json!({"action":"snapshot"}),v=commands.recv()=>{let Some(v)=v else{break;};v}};
                match jobs::request(&root, &id, value).await {
                    Ok(v) => {
                        if v.get("screen").is_some() && updates.send(Ok(v)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = updates.send(Err(format!("{e:#}"))).await;
                        break;
                    }
                }
            }
        });
        self.workbench.terminal = Some(TerminalLink { tx, rx, task });
        self.workbench.dimensions = (0, 0);
        Ok(())
    }
    pub fn terminal_input(&mut self, value: Value) -> Result<()> {
        self.workbench
            .terminal
            .as_ref()
            .context("Attach to a running job first")?
            .tx
            .try_send(value)
            .context("Terminal input queue is full; wait for the program")?;
        Ok(())
    }
    pub fn toggle_required(&mut self, name: String) -> Result<()> {
        self.launch_project("Saving verification plan", move |p, _| {
            if p.requirements()?.iter().any(|r| r.name == name) {
                p.remove_requirement(&name)?;
            } else {
                p.set_requirement(&Requirement {
                    name: name.clone(),
                    description: format!("{name} must pass on current files"),
                    check_name: name,
                })?;
            }
            Ok(JobResult::Saved("Verification plan saved.".into()))
        })
    }
}
