use super::{app::*, input::Editor};
use crate::{
    hardware,
    project::{Change, CheckResult, CheckSpec, Policy, TaskState},
    sandbox,
};
use anyhow::{Context, Result, ensure};

#[derive(Debug, Clone, Default)]
pub struct TaskView {
    pub task: Option<TaskState>,
    pub changes: Vec<Change>,
    pub checks: Vec<CheckSpec>,
    pub latest: Option<CheckResult>,
    pub verification: Option<crate::project_services::Verification>,
    pub announce_verification: bool,
}

pub(super) fn check_status(check: &CheckResult) -> String {
    if check.cancelled {
        "Stopped by you".into()
    } else if check.timed_out {
        "Stopped: time limit reached".into()
    } else if check.error.is_some() {
        "Could not complete the check".into()
    } else if check.exit_code == Some(0) {
        "Passed".into()
    } else if let Some(code) = check.exit_code {
        format!("Failed (exit code {code})")
    } else {
        "Stopped before a result was available".into()
    }
}

fn check_evidence(check: &CheckResult) -> String {
    let mut text = format!(
        "{} · {:.2} seconds\nCommand: {}\nRuns in: {}\n\nThis result describes this check on its saved files. Task progress shows whether those files are still current.\n\n",
        check_status(check),
        check.elapsed_ms as f64 / 1000.0,
        check.argv.join(" "),
        check.isolation
    );
    if let Some(error) = &check.error {
        text.push_str(&format!("What happened:\n{error}\n\n"));
    }
    text.push_str("Actual check output:\n");
    text.push_str(if check.output.is_empty() {
        "(No output was produced.)"
    } else {
        &check.output
    });
    if check.output_truncated {
        text.push_str(
            "\n\nOutput exceeded the capture limit. Only the captured part is available.",
        );
    }
    text.push_str(&format!(
        "\n\nEvidence ID: {}\nFile snapshot: {}\nExport saves the full structured record.",
        check.id, check.snapshot
    ));
    text
}
impl App {
    pub fn new_practice(&mut self) -> Result<()> {
        ensure!(
            !self.busy && !self.connecting,
            "Stop the current task before opening a practice project"
        );
        let root = self.root.clone();
        self.launch_io("Creating your practice project", move || {
            Ok(JobResult::Practice(crate::practice::create(&root)?))
        })
    }
    pub fn practice_guide(&mut self) {
        self.dialog = Some(Dialog::Menu {
            title: "Your first repair".into(),
            description: "The example has a real bug. Start with its failing check, repair greeting.py, check all four cases, then try Undo in Task. Python 3 is required; a model is optional for manual edits. Your selected access mode is preserved.".into(),
            items: vec![
                ("1. See the failing check".into(), "Run Practice behavior; inspect expected and actual results".into(), MenuAction::RunCheck("Practice behavior".into())),
                ("2. Inspect or edit greeting.py".into(), "Open Project files; Alt saves a checkpoint for edits".into(), MenuAction::Page(Page::Files)),
                ("3. Ask your selected model".into(), "README.md contains the practice goal; connect a model if needed".into(), MenuAction::Page(Page::Chat)),
                ("4. Check the repair and try Undo".into(), "Task shows evidence and tracked changes; rerun after undo".into(), MenuAction::Page(Page::Task)),
                ("Start another practice project".into(), "Create a fresh example; keep this project and its history".into(), MenuAction::NewPractice),
            ], selected: 0,
        });
    }
    pub fn refresh_task(&mut self) -> Result<()> {
        self.refresh_task_with_notice(false)
    }
    pub(super) fn refresh_task_with_notice(&mut self, announce_verification: bool) -> Result<()> {
        let preferred = self.session.as_ref().map(|s| s.id.clone());
        self.launch_project("Reading task progress", move |p, _| {
            let task = preferred.or(p.latest_task()?);
            Ok(JobResult::Task(Box::new(TaskView {
                announce_verification,
                verification: Some(p.verification()?),
                task: task.as_ref().and_then(|t| p.task(t).ok()),
                changes: p.changes(task.as_deref())?,
                checks: p.checks()?,
                latest: task
                    .as_ref()
                    .map(|t| p.latest_check(t))
                    .transpose()?
                    .flatten(),
            })))
        })
    }
    pub fn task_action(&mut self, action: &str) -> Result<()> {
        match action {
            "task"=>{self.set_page(Page::Task);if self.job.is_none(){self.refresh_task()?;}},
            "task-refresh"=>self.refresh_task()?,
            "task-diff"=>{
                let c=self.task_view.changes.get(self.change_selected).context("No checkpoint selected yet")?;
                self.dialog=Some(Dialog::Notice{title:format!("{} · {}",c.path,c.status),body:format!("{}\n{}\nCheckpoint {}\n{}",c.reason,if c.test_change{"This change affects a test/assertion. Review it separately."}else{"Saved file versions are used for undo."},c.id,c.diff()),scroll:0});
            },
            "task-undo"=>{
                ensure!(!self.busy&&!self.connecting&&self.job.is_none(),"Stop the current task before undoing a change");
                let c=self.task_view.changes.get(self.change_selected).context("Select an applied change first")?;
                ensure!(c.status=="applied","Select an applied change; this action is already {}",c.status);
                self.dialog=Some(Dialog::Confirm{title:format!("Undo change to {}?",c.path),body:format!("Alt will restore the saved contents before this action. If you changed the file afterward, undo stops and preserves your work.\n\n{}\n\n{}",c.reason,c.diff()),action:ConfirmAction::Undo(c.id.clone()),selected:0});
            },
            "task-undo-all"=> {
                ensure!(!self.busy&&!self.connecting&&self.job.is_none(),"Stop the current task before undo");
                self.launch_project("Preparing task undo", |_,id| Ok(JobResult::Dialog(Dialog::Confirm{title:"Undo this task's file edits?".into(),body:"Restores tracked file edits in reverse order after checking for conflicts. Terminal commands and external side effects are not undone.".into(),action:ConfirmAction::UndoTask(id),selected:0})))?;
            },
            "task-check"=> {
                ensure!(!self.busy&&!self.connecting,"Wait for or stop the current operation before running checks");
                let policy = self.preferences.access_policy;
                self.launch_project("Reading available checks", move |p,_| {
                    let checks = p.checks()?;
                    if checks.is_empty() { return Ok(JobResult::Dialog(check_setup(p))); }
                    Ok(JobResult::Dialog(Dialog::Menu{title:"Run checks again".into(),description:"Runs immediately after your selection and records the exact file snapshot.".into(),items:checks.into_iter().map(|s|(s.name.clone(),format!("{:?} · {} · {}s · {}",s.contract.kind,s.argv.join(" "),s.timeout_secs,policy.label()),MenuAction::RunCheck(s.name))).collect(),selected:0}))
                })?;
            },
            "task-configure"=> {
                self.launch_project("Finding project checks", |p,_| {
                    Ok(JobResult::Dialog(check_setup(p)))
                })?;
            },
            "task-memory"=>self.dialog=Some(Dialog::Input{title:"Search project memory and files".into(),hint:"Enter a topic, function name, or error. Current files are indexed locally; no second model is needed.".into(),editor:Editor::new("project"),action:InputAction::MemorySearch,multiline:false}),
            "task-note"=>self.dialog=Some(Dialog::Input{title:"Remember a decision".into(),hint:"Describe a requirement or decision to keep across future messages. Ctrl+S saves.".into(),editor:Editor::default(),action:InputAction::TaskNote,multiline:true}),
            "task-evidence"=>{let c=self.task_view.latest.as_ref().context("No checks recorded yet. Configure and run a check first")?;self.dialog=Some(Dialog::Notice{title:format!("Check evidence · {}",c.name),body:check_evidence(c),scroll:0});},
            "task-export"=> {
                let root = self.root.clone();
                self.launch_project("Exporting project evidence", move |p,_| {
                    let path=root.join("exports").join(format!("project-{}.json",uuid::Uuid::new_v4()));
                    crate::config::private_dir(path.parent().context("Export directory")?)?;
                    crate::config::atomic_write(&path,&serde_json::to_vec_pretty(&p.export()?)?)?;
                    Ok(JobResult::Notice("Project evidence exported".into(),format!("{}\nIncludes saved file versions, check output, contracts, task notes and sources.",path.display())))
                })?;
            },
            "access"=>{
                ensure!(!self.busy&&!self.connecting,"Stop the current task before changing access");
                self.dialog=Some(Dialog::Menu{title:"Choose the access you want".into(),description:"The selected model stays the same. Each mode supports ordinary conversation, project inspection and durable memory.".into(),items:vec![
                    ("Guided changes".into(),"Review edits with undo; run checks with filesystem/network isolation".into(),MenuAction::Access(Policy::Guided)),
                    ("Full access".into(),"Arbitrary terminal commands, network, installs and external tools with your OS permissions".into(),MenuAction::Access(Policy::Trusted)),
                    ("Review only".into(),"Inspect and explain; no file edits or code execution".into(),MenuAction::Access(Policy::ReviewOnly)),
                ],selected:0});
            },
            "hardware"=>self.launch_job("Inspecting this computer",|_,_|async {Ok(JobResult::Hardware(hardware::inspect().await))})?,
            "evaluate"=>{
                ensure!(!self.busy&&!self.connecting,"Stop the current task before checking the model");
                let (_,profile)=self.current_profile().context("Choose a model first")?;ensure!(profile.uncensored,"This evaluation requires an explicitly selected uncensored/abliterated model; select one from Models first");
                self.dialog=Some(Dialog::Confirm{title:"Check this model's tool use?".into(),body:format!("{}\n\nRuns a harmless arithmetic tool call, then asks the model to use its actual result. May load your local model. This tests the exact selected model and context; it does not prove general coding quality.",profile.model),action:ConfirmAction::Evaluate,selected:0});
            },
            _=>{}
        }
        Ok(())
    }
    pub fn run_project_check(&mut self, name: String) -> Result<()> {
        ensure!(
            !self.busy && !self.connecting,
            "Wait for or stop the model before running checks"
        );
        let preferred = self.session.as_ref().map(|s| s.id.clone());
        let root = self.root.clone();
        let cwd = self.preferences.project.clone();
        let policy = self.preferences.access_policy;
        self.page = Page::Task;
        self.nav_index = 7;
        self.launch_job("Running project checks", move |cancel, _| async move {
            let task =
                crate::project_worker::run(root.clone(), cwd.clone(), cancel.clone(), move |p| {
                    crate::project_worker::task(p, preferred)
                })
                .await?;
            Ok(JobResult::Check(
                sandbox::run(root, cwd, task, name, policy, cancel).await?,
            ))
        })
    }
    pub fn set_access(&mut self, policy: Policy) -> Result<()> {
        ensure!(!self.busy && !self.connecting, "Stop the task first");
        self.preferences.access_policy = policy;
        self.preferences.save(&self.root)?;
        self.workspace = None;
        self.connected = false;
        self.trust_session = false;
        self.notify(format!(
            "Access saved: {}. Your next message reconnects with this mode.",
            policy.label()
        ));
        Ok(())
    }
    pub fn apply_hardware_choice(&mut self, context: u32) -> Result<()> {
        ensure!(
            !self.busy && !self.connecting,
            "Stop the task before changing context"
        );
        self.preferences.context_tokens = context;
        self.preferences.save(&self.root)?;
        if let Some(p) = self.config.profiles.get_mut(&self.config.default_profile) {
            p.context_tokens = context;
            self.config.save(&self.root)?;
        }
        self.notify(format!("Selected {context} context tokens for new conversations. Use Check model to test the selected configuration."));
        Ok(())
    }
}

fn check_setup(p: &crate::project::Project) -> Dialog {
    let mut items = sandbox::suggested(&p.root)
        .into_iter()
        .map(|s| {
            (
                s.name.clone(),
                format!("{} · requires installed dependencies", s.argv.join(" ")),
                MenuAction::ConfigureCheck(s),
            )
        })
        .collect::<Vec<_>>();
    items.push((
        "Enter a check command".into(),
        "Register your project's documented command".into(),
        MenuAction::CustomCheck,
    ));
    items.push((
        "Set check purpose and evidence".into(),
        "Tests, build, lint, health or custom; pin an independent assertion".into(),
        MenuAction::Manager {
            action: "check-contract".into(),
            state: serde_json::json!({}),
        },
    ));
    Dialog::Menu{title:"Choose what should prove the change works".into(),description:"Choose a suggested command or enter the project's documented check. Saving does not run it. Set its purpose and evidence before treating it as behavioral proof.".into(),items,selected:0}
}
