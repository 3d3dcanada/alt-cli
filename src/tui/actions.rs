//! Discoverable global actions back keyboard shortcuts and the command palette.
//! Form-only controls live with their dialogs; they cannot run after a palette closes.
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
        "connect",
        "Add connection",
        "Connect Ollama, LM Studio, another API or a local model file"
    ),
    action!(
        "practice",
        "Try a practice project",
        "Learn editing, checks and undo in a fresh example folder"
    ),
    action!(
        "import",
        "Import model file",
        "Choose a GGUF file already on this computer"
    ),
    action!(
        "hub-search",
        "Find a model",
        "Search downloadable models on Hugging Face"
    ),
    action!(
        "hub-repo",
        "Open model repository",
        "Enter an exact Hugging Face publisher/model-name"
    ),
    action!(
        "install-engine",
        "Install agent engine",
        "Install the local component that gives your model tools"
    ),
    action!(
        "runtime",
        "Local model runtime",
        "Install or check the component that runs GGUF models"
    ),
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
    action!(
        "refresh-models",
        "Refresh models",
        "Reload the currently selected model list"
    ),
    action!(
        "use-model",
        "Use selected model",
        "Select or download the highlighted model"
    ),
    action!(
        "variant-filter",
        "Toggle model variant filter",
        "Include all models or filter by publisher uncensored/abliterated claims"
    ),
    action!(
        "hub-back",
        "Back to model repositories",
        "Return from repository files to search results"
    ),
    action!(
        "use-connection",
        "Use selected connection",
        "Select the highlighted connection for new conversations"
    ),
    action!(
        "edit-connection",
        "Edit selected connection",
        "Change the highlighted server's name, address or key variable"
    ),
    action!(
        "test-connection",
        "Test selected connection",
        "Check the highlighted server and list its models"
    ),
    action!(
        "remove-connection",
        "Remove selected connection",
        "Review before removing settings; keep models and conversations"
    ),
    action!(
        "resume",
        "Resume selected conversation",
        "Continue the highlighted saved conversation"
    ),
    action!(
        "rename",
        "Rename selected conversation",
        "Give the highlighted conversation a useful title"
    ),
    action!(
        "archive",
        "Archive or restore conversation",
        "Review changing the highlighted conversation's archive status"
    ),
    action!(
        "show-archived",
        "Show archived conversations",
        "Switch between current and archived saved conversations"
    ),
    action!(
        "search-sessions",
        "Search conversations",
        "Find saved conversations by title, project or model"
    ),
    action!(
        "file-view",
        "View selected file",
        "Read the highlighted project file"
    ),
    action!(
        "file-edit",
        "Edit selected file",
        "Preview changes and keep an undo checkpoint"
    ),
    action!(
        "file-new",
        "Create project file",
        "Choose a new file path, then preview its contents before saving"
    ),
    action!(
        "files-search",
        "Filter project files",
        "Find project files by their path"
    ),
    action!(
        "file-find",
        "Find text in selected file",
        "Search inside the highlighted file"
    ),
    action!(
        "file-line",
        "Go to line in selected file",
        "Read a particular part of the highlighted file"
    ),
    action!(
        "files-refresh",
        "Refresh project files",
        "Update the project file list"
    ),
    action!(
        "all-diffs",
        "Review project changes",
        "Inspect tracked changes and their history"
    ),
    action!(
        "job-new",
        "Start terminal command",
        "Run a command or interactive program using Full access"
    ),
    action!(
        "job-attach",
        "Attach to selected terminal",
        "Send keyboard input to the highlighted terminal job"
    ),
    action!(
        "job-stop",
        "Stop selected terminal job",
        "Stop the highlighted running command"
    ),
    action!(
        "job-restart",
        "Restart selected terminal job",
        "Run the highlighted command again using Full access"
    ),
    action!(
        "job-logs",
        "Read selected terminal output",
        "Inspect retained output from the highlighted job"
    ),
    action!(
        "job-health",
        "Check selected service",
        "Check the highlighted terminal service's HTTP address"
    ),
    action!(
        "jobs-refresh",
        "Refresh terminal jobs",
        "Update this project's command list"
    ),
    action!(
        "task-refresh",
        "Refresh task progress",
        "Update current changes and check results"
    ),
    action!(
        "task-diff",
        "Review selected change",
        "See exactly what the highlighted edit changed"
    ),
    action!(
        "task-undo",
        "Undo selected change",
        "Review restoring the highlighted tracked edit"
    ),
    action!(
        "task-export",
        "Export task evidence",
        "Save this task's changes and check records"
    ),
    action!(
        "task-memory",
        "Search project memory",
        "Find relevant files, symbols, decisions or errors"
    ),
    action!(
        "task-note",
        "Remember a decision",
        "Save a project decision for future messages"
    ),
    action!(
        "task-evidence",
        "Review latest check",
        "Inspect actual output and status from the latest check"
    ),
    action!(
        "verification-plan",
        "Choose required checks",
        "Select which project checks must pass"
    ),
    action!(
        "verification-status",
        "Review required checks",
        "Inspect whether required checks passed on current files"
    ),
    action!(
        "verification-run",
        "Run required checks",
        "Run every required check and retain the results"
    ),
    action!(
        "pin-requirement",
        "Pin a requirement",
        "Keep an important project instruction in future context"
    ),
    action!(
        "unpin-requirement",
        "Remove a pinned requirement",
        "Choose a pinned instruction to remove; keep conversation history"
    ),
    action!(
        "context-refresh",
        "Refresh project memory",
        "Update the displayed brief, requirements and model context"
    ),
    action!(
        "access",
        "Choose access mode",
        "Choose how Alt may use files, terminal commands and tools"
    ),
    action!(
        "hardware",
        "Inspect this computer",
        "Review memory and hardware recommendations"
    ),
    action!(
        "evaluate",
        "Evaluate selected model",
        "Review a practical model capability check before running it"
    ),
    action!(
        "manage-tools",
        "Tools and workflows",
        "Choose tool packs, external tools and reusable skills"
    ),
    action!(
        "manage-storage",
        "Storage and recovery",
        "Review backups, diagnostics, archives and disk use"
    ),
    action!(
        "manage-models",
        "Model files and cache",
        "Manage downloaded models, cache location and Hub access"
    ),
    action!(
        "runtime-settings",
        "Managed runtime settings",
        "Review model loading, CPU, GPU and memory choices"
    ),
    action!(
        "benchmark-preview",
        "Measure model speed",
        "Review a context and performance measurement before running it"
    ),
    action!(
        "tools-focus",
        "Choose tool focus",
        "Select tools suited to the work you want to do"
    ),
    action!(
        "qualify-preview",
        "Check model readiness",
        "Review checks of the selected model and runtime combination"
    ),
    action!(
        "quit",
        "Quit Alt",
        "Save drafts and close Alt; confirm if work is active"
    ),
];
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
