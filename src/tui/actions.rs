//! One discoverable action catalog backs page buttons, shortcuts and the palette.
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
#[derive(Clone, Copy)]
pub struct Action {
    pub id: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub shortcut: Option<char>,
}
macro_rules! action {
    ($id:literal, $label:literal, $hint:literal) => {
        Action {
            id: $id,
            label: $label,
            hint: $hint,
            shortcut: None,
        }
    };
    ($id:literal, $label:literal, $hint:literal, $key:literal) => {
        Action {
            id: $id,
            label: $label,
            hint: $hint,
            shortcut: Some($key),
        }
    };
}
pub const ALL: &[Action] = &[
    action!(
        "check-inputs",
        "Check inputs and generated files",
        "Declare external inputs and new generated output paths for a configured check"
    ),
    action!(
        "check-logs",
        "Retained check output",
        "Inspect integrity-checked stdout and stderr, including late failures"
    ),
    action!(
        "model-fit",
        "Fit selected model",
        "Estimate RAM, VRAM and context choices; apply only after review"
    ),
    action!(
        "storage-usage",
        "Storage usage",
        "Inspect evidence growth and explicit retention options"
    ),
    action!(
        "storage-budget",
        "Evidence storage budget",
        "Set the warning threshold without deleting files"
    ),
    action!(
        "inference-export",
        "Export inference evidence",
        "Create a verified archive for a selected connection"
    ),
    action!(
        "recovered-files",
        "Recovered file versions",
        "Review and restore a file version displaced by a conflict"
    ),
    action!("page-home", "Home", "Resume the next setup or project step"),
    action!(
        "page-chat",
        "Workspace",
        "Describe your goal and continue a task"
    ),
    action!(
        "page-models",
        "Models",
        "Import, find and select model files"
    ),
    action!(
        "page-connections",
        "Connections",
        "Configure or test a model server"
    ),
    action!(
        "page-sessions",
        "Conversations",
        "Resume saved conversations"
    ),
    action!(
        "page-settings",
        "Settings",
        "Model, tools, hardware and access"
    ),
    action!("page-help", "Help", "Keyboard and first-project guide"),
    action!(
        "task",
        "Task progress",
        "Inspect actual changes, checks and undo"
    ),
    action!(
        "page-files",
        "Project files",
        "View and edit project source"
    ),
    action!(
        "page-jobs",
        "Terminal jobs",
        "Start or inspect terminal commands"
    ),
    action!(
        "page-context",
        "Context & memory",
        "Inspect requirements and project memory"
    ),
    action!(
        "new",
        "New",
        "Keep this draft and start a new conversation",
        'n'
    ),
    action!(
        "project",
        "Choose project",
        "Browse to the folder you want to work on"
    ),
    action!("brief", "Brief", "Edit the saved project brief", 'b'),
    action!("send", "Send", "Submit the current message"),
    action!("stop", "Stop", "Stop the current connection or task"),
    action!(
        "reconnect",
        "Reconnect",
        "Resume this saved task on a new connection",
        'r'
    ),
    action!(
        "allowance",
        "Allowance",
        "Inspect, extend or recover the connection allowance",
        'l'
    ),
    action!(
        "templates",
        "Starting tasks",
        "Choose an editable example request"
    ),
    action!(
        "export",
        "Export",
        "Export this conversation and its evidence"
    ),
    action!(
        "drafts",
        "Saved drafts",
        "Restore or explicitly discard a retained draft",
        'd'
    ),
    action!(
        "save-drafts",
        "Save drafts",
        "Flush current text to durable local storage"
    ),
    action!(
        "understanding",
        "What Alt currently understands",
        "Inspect exact active user requirements and corrections"
    ),
    action!(
        "correct-requirement",
        "Correct a requirement",
        "Explicitly replace a user request and retain its history"
    ),
    action!(
        "next-step",
        "Continue next step",
        "Resume the first-project guide"
    ),
    action!(
        "manual-model",
        "Enter model ID",
        "Use an exact model name without inventory",
        'o'
    ),
    action!(
        "test-form",
        "Check connection",
        "Check the server and choose its model"
    ),
    action!(
        "task-configure",
        "Prepare project checks",
        "Choose the actual command that should prove the work"
    ),
    action!("task-check", "Run checks", "Run configured project checks"),
    action!(
        "task-undo-all",
        "Undo task",
        "Review restoring this task's tracked edits"
    ),
];
pub fn get(id: &str) -> Option<&'static Action> {
    ALL.iter().find(|a| a.id == id)
}
pub fn search(query: &str) -> Vec<&'static Action> {
    let normalized = query.trim().to_lowercase();
    let words: Vec<_> = normalized.split_whitespace().collect();
    let mut found: Vec<_> = ALL
        .iter()
        .filter(|a| {
            let text = format!("{} {} {}", a.label, a.hint, a.id).to_lowercase();
            words.iter().all(|w| text.contains(w))
        })
        .collect();
    if !normalized.is_empty() {
        found.sort_by_key(|a| {
            let label = a.label.to_lowercase();
            (label != normalized, !label.starts_with(&normalized))
        });
    }
    found
}
pub fn shortcut(key: KeyEvent) -> Option<&'static str> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    let KeyCode::Char(c) = key.code else {
        return None;
    };
    ALL.iter().find(|a| a.shortcut == Some(c)).map(|a| a.id)
}
