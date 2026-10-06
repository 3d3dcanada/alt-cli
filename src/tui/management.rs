//! Labelled settings workflows; commands remain available for advanced users.
use super::{app::*, input::Editor};
use crate::{extensions, models, packs, project::Policy, storage};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
impl App {
    pub(super) fn manager_menu(
        &mut self,
        title: &str,
        description: &str,
        items: Vec<(String, String, String, Value)>,
    ) {
        self.dialog = Some(Dialog::Menu {
            title: title.into(),
            description: description.into(),
            items: items
                .into_iter()
                .map(|(a, b, action, state)| (a, b, MenuAction::Manager { action, state }))
                .collect(),
            selected: 0,
        });
    }
    pub(super) fn manager_input(
        &mut self,
        title: &str,
        hint: &str,
        action: &str,
        state: Value,
        initial: &str,
    ) {
        self.dialog = Some(Dialog::Input {
            title: title.into(),
            hint: hint.into(),
            action: InputAction::Manager {
                action: action.into(),
                state,
            },
            editor: Editor::new(initial),
            multiline: false,
        });
    }
    pub(super) fn manager_confirm(
        &mut self,
        title: &str,
        body: String,
        action: &str,
        state: Value,
    ) {
        self.dialog = Some(Dialog::Confirm {
            title: title.into(),
            body,
            action: ConfirmAction::Manager {
                action: action.into(),
                state,
            },
            selected: 0,
        });
    }
    pub fn manager_action(&mut self, action: &str, state: Value) -> Result<()> {
        if action.starts_with("check-") {
            return self.manage_verification(action, state);
        }
        if action == "tools-focus" {
            self.manager_menu("Which tools fit this task?","This changes the schemas shown on the next connection. Full access remains an independent setting. Select All whenever the task needs a wider tool set.",
                [("All native tools","all"),("Inspect and explain","inspect"),("Read, edit and run named checks","coding"),("Terminal and file reading","terminal")].into_iter().map(|(label,profile)|(label.into(),String::new(),"tools-focus-save".into(),json!({"profile":profile}))).collect());
            return Ok(());
        }
        if action == "tools-focus-save" {
            ensure!(
                !self.busy && !self.connecting,
                "Stop the current task before changing tool focus"
            );
            self.preferences.tool_profile = serde_json::from_value(state["profile"].clone())?;
            self.preferences.save(&self.root)?;
            self.workspace = None;
            self.connected = false;
            self.notify("Tool focus saved. Your next message connects with the selected tools.");
            return Ok(());
        }
        if action == "qualify-history" {
            let root = self.root.clone();
            let model = self
                .current_profile()
                .context("Choose a model")?
                .1
                .model
                .clone();
            self.launch_io("Reading saved compatibility measurements", move || {
                Ok(JobResult::Notice(
                    "Measured compatibility".into(),
                    crate::qualification::history(&root, &model)?,
                ))
            })?;
            return Ok(());
        }
        if action == "qualify-preview" {
            self.manager_menu("Model qualification","Measure this computer or review previous results. Generation speed alone does not prove coding quality.",vec![("Run context and restart measurements".into(),"2K, 4K and 8K context; preserves your settings".into(),"qualify-confirm".into(),json!({})),("Review saved measurements".into(),"Inspect passed and failed attempts for this model".into(),"qualify-history".into(),json!({}))]);
            return Ok(());
        }
        if action == "qualify-confirm" {
            let (_, profile) = self.current_profile().context("Choose a model first")?;
            ensure!(
                profile.uncensored,
                "Select an explicitly marked uncensored/abliterated model first"
            );
            self.manager_confirm("Measure this model on your computer?",format!("{}\nContexts: 2048, 4096, 8192. Three generation samples per context plus owned-runtime stop and restart. May take several minutes. Settings and selected model stay unchanged; each attempt is saved.",profile.model),"qualify-run",json!({}));
            return Ok(());
        }
        if action == "qualify-run" {
            ensure!(
                !self.busy && !self.connecting,
                "Stop the current task before qualification"
            );
            let root = self.root.clone();
            let prefs = self.preferences.clone();
            let profile = self.current_profile().context("Choose a model")?.1.clone();
            self.launch_job(
                "Qualifying selected model contexts",
                move |cancel, _| async move {
                    let result = crate::qualification::run(
                        &root,
                        &prefs,
                        &profile,
                        &[2048, 4096, 8192],
                        1,
                        cancel,
                    )
                    .await?;
                    Ok(JobResult::Notice(
                        "Qualification evidence".into(),
                        crate::qualification::summary(&result),
                    ))
                },
            )?;
            return Ok(());
        }
        match action {
            "language-start" | "language-file" | "language-position" | "language-name"
            | "language-query" | "language-result" | "language-apply" => {
                self.manage_language(action, state)
            }
            "runtime-settings"
            | "runtime-field"
            | "runtime-save"
            | "benchmark-preview"
            | "benchmark-run"
            | "runtime-thinking"
            | "runtime-thinking-save" => self.manage_runtime(action, state),
            "manage-storage"
            | "storage-usage"
            | "diagnostics"
            | "storage-report"
            | "save-diagnostics"
            | "backup"
            | "backup-run"
            | "restore"
            | "restore-destination"
            | "restore-preview"
            | "restore-run"
            | "history-days"
            | "history-preview"
            | "history-preview-result"
            | "history-run"
            | "history-restore"
            | "history-restore-run"
            | "retention"
            | "retention-preview"
            | "retention-preview-result"
            | "retention-run" => self.manage_storage(action, state),
            "manage-models"
            | "hub-auth-help"
            | "cache-destination"
            | "cache-run"
            | "model-remove-preview"
            | "model-remove" => self.manage_models(action, state),
            "extensions"
            | "extension-new"
            | "extension-transport"
            | "extension-command"
            | "extension-url"
            | "extension-auth"
            | "extension-save-http"
            | "extension-save-stdio"
            | "extension-menu"
            | "extension-probe"
            | "extension-tools"
            | "extension-toggle"
            | "extension-disable"
            | "extension-remove" => self.manage_extensions(action, state),
            "manage-tools" | "packs" | "pack-configure" | "pack-url" | "pack-selector"
            | "pack-contains" | "pack-requirements" | "git-configure" | "git-branch"
            | "git-paths" | "git-message" | "pack-preview" | "pack-run" | "findings"
            | "finding-select" | "finding-preview" | "finding-run" | "findings-export" => {
                self.manage_workflows(action, state)
            }
            _ => anyhow::bail!("Unknown management action {action}"),
        }
    }
    fn manage_language(&mut self, action: &str, mut state: Value) -> Result<()> {
        let value = state["value"].as_str().unwrap_or("").to_owned();
        match action {
            "language-start" => self.manager_input(
                "Language server executable",
                "For example rust-analyzer or an absolute server path. It runs with Full access.",
                "language-file",
                json!({}),
                "rust-analyzer",
            ),
            "language-file" => self.manager_input(
                "Source file",
                "Relative path inside this project.",
                "language-position",
                json!({"server":value}),
                "",
            ),
            "language-position" => {
                state["path"] = json!(value);
                self.manager_input(
                    "Identifier position",
                    "Line:column, both starting at 1. Columns count UTF-16 units.",
                    "language-name",
                    state,
                    "1:1",
                );
            }
            "language-name" => {
                let (line, column) = value.split_once(':').context("Use line:column")?;
                state["line"] = json!(line.parse::<u32>()?);
                state["column"] = json!(column.parse::<u32>()?);
                self.manager_input(
                    "New name, or blank for references",
                    "A rename opens a multi-file diff before any file changes.",
                    "language-query",
                    state,
                    "",
                );
            }
            "language-query" => {
                state["rename"] = json!(value);
                let root = self.root.clone();
                let cwd = self.preferences.project.clone();
                let policy = self.preferences.access_policy;
                self.launch_job("Querying language server",move|cancel,_|async move{let server=std::path::PathBuf::from(state["server"].as_str().context("Server")?);let name=state["rename"].as_str().unwrap_or("");let operation=crate::language::query(&root,&cwd,crate::language::Query{server:&server,args:&[],path:state["path"].as_str().context("Path")?,line:state["line"].as_u64().context("Line")? as u32,column:state["column"].as_u64().context("Column")? as u32,new_name:(!name.is_empty()).then_some(name)},policy);let result=tokio::select!{r=operation=>r?,_=models::cancelled(&cancel)=>anyhow::bail!("Language-server query cancelled; no source edits applied")};Ok(JobResult::Manager("language-result".into(),result))})?;
            }
            "language-result" => {
                if let Some(changes) = state["changes"].as_array() {
                    let changes: Vec<crate::project::Change> = changes
                        .iter()
                        .map(|c| serde_json::from_value(c.clone()))
                        .collect::<std::result::Result<_, _>>()?;
                    let diff = changes
                        .iter()
                        .map(|c| format!("{}\n{}\n", c.path, c.diff()))
                        .collect::<String>();
                    self.manager_confirm("Apply this identifier rename?",format!("Each edit has a checkpoint. Stale files are rejected before application.\n\n{diff}"),"language-apply",json!({"ids":changes.iter().map(|c|&c.id).collect::<Vec<_>>()}));
                } else {
                    self.dialog = Some(Dialog::Notice {
                        title: "Language-server references".into(),
                        body: serde_json::to_string_pretty(&state)?,
                        scroll: 0,
                    });
                }
            }
            "language-apply" => {
                let ids: Vec<String> = serde_json::from_value(state["ids"].clone())?;
                let root = self.root.clone();
                let cwd = self.preferences.project.clone();
                let policy = self.preferences.access_policy;
                self.launch_io("Applying identifier rename", move || {
                    crate::language::apply(&root, &cwd, &ids, policy)?;
                    Ok(JobResult::Saved("Rename applied with checkpoints. Run project checks before declaring it complete.".into()))
                })?;
            }
            _ => anyhow::bail!("Unknown management action {action}"),
        }
        Ok(())
    }
    fn manage_runtime(&mut self, action: &str, state: Value) -> Result<()> {
        let value = state["value"].as_str().unwrap_or("").to_owned();
        match action {
            "runtime-settings" => {
                let settings = &self.preferences.runtime;
                self.manager_menu("Managed runtime settings",&format!("GPU layers {} · threads {} (0 auto) · batch {} · K/V {} / {}",settings.gpu_layers,settings.threads,settings.batch,settings.cache_k,settings.cache_v),vec![("GPU layers".into(),"0 CPU; -1 all layers. Requires compatible runtime/GPU.".into(),"runtime-field".into(),json!({"field":"gpu_layers","current":settings.gpu_layers.to_string()})),("CPU threads".into(),"0 uses automatic conservative choice.".into(),"runtime-field".into(),json!({"field":"threads","current":settings.threads.to_string()})),("Batch size".into(),"Lower reduces working memory; 128 is the default.".into(),"runtime-field".into(),json!({"field":"batch","current":settings.batch.to_string()})),("K cache".into(),"f16, q8_0 or q4_0; test exact runtime support.".into(),"runtime-field".into(),json!({"field":"cache_k","current":settings.cache_k})),("V cache".into(),"Quantization requires compatible flash attention.".into(),"runtime-field".into(),json!({"field":"cache_v","current":settings.cache_v})),("Flash attention".into(),"true or false; hardware support varies.".into(),"runtime-field".into(),json!({"field":"flash_attention","current":settings.flash_attention.to_string()})),("Reasoning mode".into(),format!("Current: {}. Only applies to templates with enable_thinking.",settings.thinking.map(|v|if v{"on"}else{"off"}).unwrap_or("model default")),"runtime-thinking".into(),json!({}))]);
            }
            "runtime-thinking" => self.manager_menu("Choose the model's reasoning mode","Only templates that use enable_thinking honor this setting. Off may reduce long reasoning output; compare independent task results before relying on it.",vec![("Model default".into(),"No override".into(),"runtime-thinking-save".into(),json!({"thinking":null})),("Reasoning on".into(),"May use more output tokens and time".into(),"runtime-thinking-save".into(),json!({"thinking":true})),("Reasoning off".into(),"Test task accuracy with shorter reasoning output".into(),"runtime-thinking-save".into(),json!({"thinking":false}))]),
            "runtime-thinking-save" => {
                ensure!(!self.busy&&!self.connecting,"Stop the current task first");
                self.preferences.runtime.thinking=state["thinking"].as_bool();self.preferences.save(&self.root)?;
                self.workspace=None;self.connected=false;self.notify("Reasoning choice saved for the next managed runtime. The selected model and access mode are unchanged.");
            }
            "runtime-field" => {
                let field = state["field"].as_str().context("Setting")?.to_owned();
                let initial = state["current"].as_str().unwrap_or("").to_owned();
                self.manager_input(
                    &format!("Set {field}"),
                    "Applies the next time the managed runtime starts; no model substitution.",
                    "runtime-save",
                    state,
                    &initial,
                );
            }
            "runtime-save" => {
                let mut s = self.preferences.runtime.clone();
                match state["field"].as_str().unwrap_or("") {
                    "gpu_layers" => s.gpu_layers = value.parse()?,
                    "threads" => s.threads = value.parse()?,
                    "batch" => s.batch = value.parse()?,
                    "cache_k" => s.cache_k = value,
                    "cache_v" => s.cache_v = value,
                    "flash_attention" => s.flash_attention = value.parse()?,
                    _ => anyhow::bail!("Unknown runtime setting"),
                };
                s.validate()?;
                self.preferences.runtime = s;
                self.preferences.save(&self.root)?;
                self.notify(
                    "Runtime settings saved. Restart the conversation connection to apply them.",
                );
            }
            "benchmark-preview" => {
                let (_, p) = self.current_profile().context("Select a model first")?;
                ensure!(
                    p.uncensored,
                    "Select and mark an uncensored/abliterated model before a live benchmark"
                );
                self.manager_confirm("Benchmark this exact model?",format!("{} · {} tokens\nLoads the managed model if selected and runs three short generation trials. Results describe this computer and do not establish coding-task accuracy.",p.model,p.context_tokens),"benchmark-run",json!({}));
            }
            "benchmark-run" => {
                let root = self.root.clone();
                let preferences = self.preferences.clone();
                let (_, p) = self.current_profile().context("Select a model")?;
                let profile = p.clone();
                self.launch_job("Measuring model performance", move |cancel, _| async move {
                    let result =
                        crate::benchmark::run(&root, &preferences, &profile, cancel).await?;
                    Ok(JobResult::Notice(
                        "Measured model report".into(),
                        serde_json::to_string_pretty(&result)?,
                    ))
                })?;
            }
            _ => anyhow::bail!("Unknown management action {action}"),
        }
        Ok(())
    }
    fn manage_storage(&mut self, action: &str, mut state: Value) -> Result<()> {
        let value = state["value"].as_str().unwrap_or("").to_owned();
        match action {
"manage-storage"=>self.manager_menu("Storage and recovery","Backups include Alt state; project files and model weights stay where they are.",vec![("Storage usage".into(),"Inspect disk use by category".into(),"storage-usage".into(),json!({})),("Create state backup".into(),"Consistent conversation and checkpoint archive".into(),"backup".into(),json!({})),("Restore state backup".into(),"Choose a new data folder; existing state stays intact".into(),"restore".into(),json!({})),("Review diagnostics".into(),"Credentials, prompts, logs and project paths omitted".into(),"diagnostics".into(),json!({})),("Archive old conversations".into(),"Only user-archived conversations; compressed recovery copy before removal".into(),"history-days".into(),json!({})),("Restore a retained conversation".into(),"Choose a .jsonl.gz recovery archive".into(),"history-restore".into(),json!({})),("Clean old reports and finished jobs".into(),"Preview first; conversations and checkpoints are preserved".into(),"retention".into(),json!({}))]),
"storage-usage"|"diagnostics"=>{let root=self.root.clone();let diagnostics=action=="diagnostics";self.launch_job("Preparing storage report",move|_,_|async move {let result=tokio::task::spawn_blocking(move||if diagnostics{storage::diagnostics(&root)}else{storage::usage(&root)}).await??;Ok(JobResult::Manager("storage-report".into(),json!({"report":result,"diagnostics":diagnostics})))})?;},
"storage-report"=>{if state["diagnostics"]==true {self.manager_confirm("Save this redacted diagnostics report?",serde_json::to_string_pretty(&state["report"])?,"save-diagnostics",state);}else{self.dialog=Some(Dialog::Notice{title:"Storage usage in bytes".into(),body:serde_json::to_string_pretty(&state["report"])?,scroll:0});}},
"save-diagnostics"=>{let path=self.root.join("exports").join(format!("diagnostics-{}.json",uuid::Uuid::new_v4()));crate::config::atomic_write(&path,&serde_json::to_vec_pretty(&state["report"])?)?;self.notify(format!("Saved {}",path.display()));},
"backup"=>self.manager_input("Save a state backup","Enter an archive path outside Alt's data folder. Existing files are never overwritten.","backup-run",json!({}),""),
"backup-run"=>{let root=self.root.clone();let to=std::path::PathBuf::from(value);self.launch_job("Backing up saved state",move|_,_|async move{let r=tokio::task::spawn_blocking(move||storage::backup(&root,&to)).await??;Ok(JobResult::Notice("Backup saved".into(),serde_json::to_string_pretty(&r)?))})?;},
"restore"=>self.manager_input("Choose a backup archive","Enter its full file path.","restore-destination",json!({}),""),
"restore-destination"=>self.manager_input("Choose a new data folder","Restore requires a new or empty folder. Project files and weights are not changed.","restore-preview",json!({"archive":value}),""),
"restore-preview"=>{state["destination"]=json!(value);self.manager_confirm("Restore this backup into a new folder?",format!("Archive: {}\nDestination: {}\nAfter restore, launch Alt with --data-dir pointing to the new folder.",state["archive"],state["destination"]),"restore-run",state);},
"restore-run"=>{let from=std::path::PathBuf::from(state["archive"].as_str().context("Archive")?);let to=std::path::PathBuf::from(state["destination"].as_str().context("Destination")?);self.launch_job("Restoring state",move|_,_|async move{let r=tokio::task::spawn_blocking(move||storage::restore(&from,&to)).await??;Ok(JobResult::Notice("State restored".into(),serde_json::to_string_pretty(&r)?))})?;},
"history-days"=>self.manager_input("Keep how many days of archived conversations?","Unarchived conversations are always preserved. Preview before removing older archived history.","history-preview",json!({}),"30"),
"history-preview"=>{let days:u64=value.parse()?;let root=self.root.clone();self.launch_io("Previewing conversation retention",move || {let report=crate::store::Store::open(&root)?.retain_archived(&root,days,false)?;Ok(JobResult::Manager("history-preview-result".into(),json!({"days":days,"report":report})))})?;},
"history-preview-result"=>self.manager_confirm("Move these conversations into recovery archives?",serde_json::to_string_pretty(&state["report"])?,"history-run",state),
"history-run"=>{ensure!(!self.busy&&!self.connecting,"Stop the active task before retaining conversation history");let root=self.root.clone();let days=state["days"].as_u64().context("Days")?;self.launch_job("Archiving old conversations",move|_,_|async move {let result=tokio::task::spawn_blocking(move||crate::store::Store::open(&root)?.retain_archived(&root,days,true)).await??;Ok(JobResult::Notice("Conversations retained in recovery archives".into(),serde_json::to_string_pretty(&result)?))})?;},
"history-restore"=>self.manager_input("Conversation archive path","A .jsonl.gz file from history-archives; existing conversations are never overwritten.","history-restore-run",json!({}),""),
"history-restore-run"=>{let root=self.root.clone();self.launch_io("Restoring archived conversation",move || {let id=crate::store::Store::open(&root)?.restore_history(std::path::Path::new(&value))?;Ok(JobResult::Saved(format!("Conversation {id} restored under archived conversations.")))})?;self.pending_sessions=true;},
"retention"=>self.manager_input("Keep how many days of reports?","Preview removes only old exports, evaluation reports and finished jobs; never checkpoints or conversations.","retention-preview",json!({}),"30"),
"retention-preview"=>{let days:u64=value.parse().context("Enter a whole number of days")?;let root=self.root.clone();self.launch_io("Previewing report retention",move || Ok(JobResult::Manager("retention-preview-result".into(),json!({"days":days,"report":storage::retain(&root,days,false)?}))))?;},
"retention-preview-result"=>self.manager_confirm("Delete the listed old reports?",serde_json::to_string_pretty(&state["report"])?,"retention-run",state),
"retention-run"=>{let root=self.root.clone();self.launch_io("Removing selected old reports",move || {storage::retain(&root,state["days"].as_u64().context("Days")?,true)?;Ok(JobResult::Saved("Old reports removed. Conversation databases, checkpoints and model files preserved.".into()))})?;},
            _=>anyhow::bail!("Unknown management action {action}"),
        }
        Ok(())
    }
    fn manage_models(&mut self, action: &str, state: Value) -> Result<()> {
        let value = state["value"].as_str().unwrap_or("").to_owned();
        match action {
"manage-models"=>{let mut items=vec![("Move managed model cache".into(),"Copy, verify and switch paths; keep originals until reviewed".into(),"cache-destination".into(),json!({})),("Private or gated Hub models".into(),"Use HF_TOKEN in Alt's environment; accept publisher access terms on the Hub".into(),"hub-auth-help".into(),json!({}))];for m in models::library(&self.root)?{items.push((format!("Remove {}",m.name),if m.source.is_some(){"Managed download; review weight deletion"}else{"Imported file; original always preserved"}.into(),"model-remove-preview".into(),json!({"id":m.id})));}self.manager_menu("Model files and cache",&format!("Active cache: {}",models::cache_root(&self.root)?.display()),items);},
"hub-auth-help"=>self.dialog=Some(Dialog::Notice{title:"Private Hub access".into(),body:"Alt reads HF_TOKEN from its environment and sends it only to https://huggingface.co. It does not store the token in profiles or diagnostic reports. For a gated repository, first accept its publisher's terms on the Hub. Open Models → Repository and enter publisher/model-name. An authenticated request still requires repository access.\n\nOn Linux, launch from a shell with HF_TOKEN already exported or configure it in your session's secret manager. Never paste a token into the repository name.".into(),scroll:0}),
"cache-destination"=>self.manager_input("Choose a model cache folder","Verified managed files are copied here. Imports and old copies are preserved.","cache-run",json!({}),""),
"cache-run"=>{let root=self.root.clone();self.launch_job("Relocating model cache",move|cancel,_|async move{let r=models::relocate(&root,std::path::Path::new(&value),cancel).await?;Ok(JobResult::Notice("Cache relocated; old copies retained".into(),serde_json::to_string_pretty(&r)?))})?;},
"model-remove-preview"=>{let m=models::artifact(&self.root,state["id"].as_str().context("Model ID")?)?;self.manager_confirm("Remove this model from the library?",format!("{}\n{}\nConnections using this model must be changed or removed first.",m.name,if m.source.is_some(){"Its managed downloaded weights will be deleted."}else{"Its imported original file will remain unchanged."}),"model-remove",state);},
"model-remove"=>{models::remove(&self.root,state["id"].as_str().context("Model ID")?,true)?;self.artifacts=models::library(&self.root)?;self.notify("Model removed; imported originals preserved.");},
            _=>anyhow::bail!("Unknown management action {action}"),
        }
        Ok(())
    }
    fn manage_extensions(&mut self, action: &str, mut state: Value) -> Result<()> {
        let value = state["value"].as_str().unwrap_or("").to_owned();
        match action {
"extensions"=>{let mut items=vec![("Add a connection".into(),"stdio command or Streamable HTTP URL".into(),"extension-new".into(),json!({}))];for c in extensions::list(&self.root)?{items.push((c.name.clone(),format!("{} · {} selected tools",if c.enabled{"enabled"}else{"disabled"},c.selected_tools.len()),"extension-menu".into(),json!({"name":c.name})));}self.manager_menu("External MCP connections","Tool discovery starts your configured server. Calls use Alt's normal approval controls.",items);},
"extension-new"=>self.manager_input("Name this tool connection","Use letters, digits, underscores or hyphens.","extension-transport",json!({}),""),
"extension-transport"=>{state=json!({"name":value});self.manager_menu("How does this server connect?","Use the command or URL documented by the server publisher.",vec![("Local command (stdio)".into(),"Starts an installed server process".into(),"extension-command".into(),state.clone()),("Remote Streamable HTTP".into(),"Connects to an HTTP(S) MCP endpoint".into(),"extension-url".into(),state)]);},
"extension-command"=>self.manager_input("Server launch command","The command runs with your normal environment and host permissions. Use environment references for credentials.","extension-save-stdio",state,""),
"extension-url"=>self.manager_input("MCP server URL","HTTP or HTTPS without query credentials. Next choose an optional bearer-token environment variable.","extension-auth",state,""),
"extension-auth"=>{state["url"]=json!(value);self.manager_input("Authentication variable (optional)","Enter a variable name, such as MY_MCP_TOKEN, not the token. Leave blank for no authentication.","extension-save-http",state,"");},
"extension-save-http"|"extension-save-stdio"=>{let name=state["name"].as_str().context("Name")?.to_owned();let transport=if action=="extension-save-http"{extensions::Transport::Http{url:state["url"].as_str().context("URL")?.into(),auth_env:(!value.trim().is_empty()).then(||value.trim().into())}}else{ensure!(!value.trim().is_empty(),"Enter a server command");extensions::Transport::Stdio{command:"/bin/bash".into(),args:vec!["-c".into(),value],env_names:vec![]}};extensions::save(&self.root,&extensions::Connection{name:name.clone(),transport,enabled:false,selected_tools:vec![]})?;self.manager_action("extension-menu",json!({"name":name}))?;},
"extension-menu"=>{let name=state["name"].as_str().context("Connection")?.to_owned();self.manager_menu(&name,"Check the server, then choose a small set of tools.",vec![("Check connection and choose tools".into(),"Starts the server; discovers its capabilities".into(),"extension-probe".into(),state.clone()),("Disable tools".into(),"Preserve configuration for later".into(),"extension-disable".into(),state.clone()),("Remove connection".into(),"Remove saved configuration".into(),"extension-remove".into(),state)]);},
"extension-probe"=>{ensure!(self.preferences.access_policy==Policy::Trusted,"Choose Full access to start external tools");let root=self.root.clone();let name=state["name"].as_str().context("Connection")?.to_owned();self.launch_job("Checking external server",move|cancel,_|async move{tokio::select!{r=extensions::probe(&root,&name)=>{r?;},_=models::cancelled(&cancel)=>anyhow::bail!("Connection check cancelled")};Ok(JobResult::Manager("extension-tools".into(),json!({"name":name})))})?;},
"extension-tools"=>{let name=state["name"].as_str().context("Connection")?;let c=extensions::load(&self.root,name)?;let inventory:Value=serde_json::from_slice(&std::fs::read(self.root.join("extensions/cache").join(format!("{name}.json")))?)?;let items=inventory["tools"].as_array().context("Tool inventory")?.iter().map(|t|{let tool=t["name"].as_str().unwrap_or("");(format!("{} {tool}",if c.selected_tools.iter().any(|s|s==tool){"[selected]"}else{"[ ]"}),t["description"].as_str().unwrap_or("").into(),"extension-toggle".into(),json!({"name":name,"tool":tool}))}).collect();self.manager_menu("Choose tools to expose","Select to toggle. Esc finishes. Changes apply to the next conversation connection.",items);},
"extension-toggle"=>{ensure!(!self.busy&&!self.connecting,"Stop the active model before changing external tools");let name=state["name"].as_str().context("Name")?;let tool=state["tool"].as_str().context("Tool")?;let mut c=extensions::load(&self.root,name)?;if c.selected_tools.iter().any(|s|s==tool){c.selected_tools.retain(|s|s!=tool);}else{c.selected_tools.push(tool.into());}extensions::select(&self.root,name,c.selected_tools)?;self.workspace=None;self.connected=false;self.manager_action("extension-tools",state)?;},
"extension-disable"|"extension-remove"=>{ensure!(!self.busy&&!self.connecting,"Stop the active model before changing external tools");let name=state["name"].as_str().context("Name")?;if action=="extension-remove"{extensions::remove(&self.root,name)?;}else{let mut c=extensions::load(&self.root,name)?;c.enabled=false;extensions::save(&self.root,&c)?;}self.workspace=None;self.connected=false;self.manager_action("extensions",json!({}))?;},
            _=>anyhow::bail!("Unknown management action {action}"),
        }
        Ok(())
    }
    fn manage_workflows(&mut self, action: &str, mut state: Value) -> Result<()> {
        let value = state["value"].as_str().unwrap_or("").to_owned();
        match action {
"manage-tools"=>self.manager_menu("Tools and workflows","Choose only the tools you need. External tools execute with Full access.",vec![("Project workflows".into(),"Build, Git, HTTP, browser and security checks".into(),"packs".into(),json!({})),("External MCP connections".into(),"Connect a server and select its exposed tools".into(),"extensions".into(),json!({})),("Findings and evidence".into(),"Review and export actual workflow results".into(),"findings".into(),json!({})),("Language-server references or rename".into(),"Optional installed language server; preview checkpoints before applying".into(),"language-start".into(),json!({}))]),
"packs"=>self.manager_menu("Project workflows","Optional scanners and browsers must be installed. Every attempt saves evidence.",packs::catalog().into_iter().map(|p|(p.id.clone(),format!("{} · needs {}",p.description,p.prerequisite),"pack-configure".into(),json!({"pack":p.id,"input":{}}))).collect()),
"pack-configure"=>{match state["pack"].as_str().context("Pack")? {
    "build"=>self.manager_menu("Choose your project's language","Runs its existing test runner. A zero-test result is incomplete when the runner exposes a count.",["rust","python","javascript"].into_iter().map(|language|(language.into(),"Use installed project dependencies".into(),"pack-preview".into(),json!({"pack":"build","input":{"language":language}}))).collect()),
    "git"=>self.manager_menu("Git project actions","Inspect existing changes before staging or committing. Selected paths only.",["status","diff","branch","stage","commit"].into_iter().map(|action|(action.into(),"Uses this project's repository".into(),"git-configure".into(),json!({"pack":"git","input":{"action":action}}))).collect()),
    "http"|"browser"=>self.manager_input("Application URL","Start a development server from Terminal jobs if needed.","pack-url",state,"http://127.0.0.1:3000/"),
    "pip-audit"=>self.manager_input("Requirements file","Relative path to your pinned Python requirements.","pack-requirements",state,"requirements.txt"),
    _=>self.manager_action("pack-preview",state)?}},
"pack-url"=>{state["input"]["url"]=json!(value);if state["pack"]=="browser"{self.manager_input("Element to check","CSS selector for a visible element. Body checks the whole page.","pack-selector",state,"body");}else{self.manager_input("Expected response text","Leave blank to check HTTP 200 only.","pack-contains",state,"");}},
"pack-selector"=>{state["input"]["selector"]=json!(value);self.manager_input("Expected visible text","Leave blank to check that the element appears.","pack-contains",state,"");},
"pack-contains"=>{state["input"]["contains"]=json!(value);self.manager_action("pack-preview",state)?;},
"pack-requirements"=>{state["input"]["requirements"]=json!(value);self.manager_action("pack-preview",state)?;},
"git-configure"=>{match state["input"]["action"].as_str().unwrap_or(""){"branch"=>self.manager_input("New branch name","Creates and switches to this branch; existing changes remain.","git-branch",state,""),"stage"|"commit"=>self.manager_input("Select paths","Separate paths with |. Unrelated staged files are not included.","git-paths",state,""),_=>self.manager_action("pack-preview",state)?}},
"git-branch"=>{state["input"]["branch"]=json!(value);self.manager_action("pack-preview",state)?;},
"git-paths"=>{state["input"]["paths"]=json!(value.split('|').map(str::trim).filter(|s|!s.is_empty()).collect::<Vec<_>>());if state["input"]["action"]=="commit"{self.manager_input("Commit message","This commits only the chosen paths. Inspect their diff first if unsure.","git-message",state,"");}else{self.manager_action("pack-preview",state)?;}},
"git-message"=>{state["input"]["message"]=json!(value);self.manager_action("pack-preview",state)?;},
"pack-preview"=>self.manager_confirm("Run this workflow?",format!("Project: {}\nWorkflow: {}\nInputs:\n{}\n\nRuns installed tools with Full access. Git write actions change the actual repository.",self.preferences.project.display(),state["pack"],serde_json::to_string_pretty(&state["input"])?),"pack-run",state),
"pack-run"=>{let root=self.root.clone();let cwd=self.preferences.project.clone();let policy=self.preferences.access_policy;self.launch_job("Running project workflow",move|cancel,_|async move{let r=packs::run(&root,&cwd,state["pack"].as_str().context("Pack")?,state["input"].clone(),policy,cancel).await?;Ok(JobResult::Notice(format!("{} · {}",r.pack,r.status),format!("{} finding(s)\n{}\n\nEvidence ID: {}\n\n{}",r.findings.len(),r.error.as_deref().unwrap_or("Workflow completed. Review its actual coverage."),r.id,serde_json::to_string_pretty(&r)?)))})?;},
"findings"=>{let mut items=vec![("Retest a finding".into(),"Repeat its exact workflow inputs and link new evidence".into(),"finding-select".into(),json!({}))];items.extend(["markdown","json","sarif"].into_iter().map(|format|(format!("Export {format}"),"Includes evidence IDs and reproduction inputs".into(),"findings-export".into(),json!({"format":format}))));self.manager_menu("Findings and reports","Reports contain actual project evidence; review before sharing.",items);},
"finding-select"=>{let mut items=Vec::new();for r in packs::runs(&self.root,&self.preferences.project)? {for f in r.findings {items.push((f.title.clone(),format!("{} · {} · evidence {}",f.severity,f.confidence,r.id),"finding-preview".into(),json!({"finding":f.id,"pack":r.pack,"input":r.input})));}}self.manager_menu("Choose a finding to retest","A retest records what is observed now; it does not erase the original finding.",items);},
"finding-preview"=>self.manager_confirm("Repeat this finding's workflow?",serde_json::to_string_pretty(&state)?,"finding-run",state),
"finding-run"=>{let root=self.root.clone();let cwd=self.preferences.project.clone();let policy=self.preferences.access_policy;self.launch_job("Retesting finding",move|cancel,_|async move {let result=packs::run(&root,&cwd,state["pack"].as_str().context("Pack")?,state["input"].clone(),policy,cancel).await?;packs::retest(&root,&cwd,state["finding"].as_str().context("Finding")?,&result.id)?;Ok(JobResult::Notice("Retest evidence linked".into(),serde_json::to_string_pretty(&result)?))})?;},
"findings-export"=>{let format=state["format"].as_str().context("Format")?;let text=packs::export(&self.root,&self.preferences.project,format)?;let path=self.root.join("exports").join(format!("findings-{}.{}",uuid::Uuid::new_v4(),format));crate::config::atomic_write(&path,text.as_bytes())?;self.dialog=Some(Dialog::Notice{title:"Findings exported".into(),body:format!("{}\n\n{text}",path.display()),scroll:0});},
            _=>anyhow::bail!("Unknown management action {action}"),
        }
        Ok(())
    }
}
