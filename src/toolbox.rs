//! Alt-owned MCP tools. The model-facing process has no execution authority:
//! it forwards requests to a private socket owned by the active Alt session.
use crate::{
    engine::Event,
    project::{self, Policy, Project},
    sandbox,
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    sync::{mpsc, oneshot},
};

/// Explicit schema focus, independent of OS access policy. Full access remains available.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    clap::ValueEnum,
)]
#[serde(rename_all = "kebab-case")]
pub enum ToolProfile {
    #[default]
    All,
    Inspect,
    Coding,
    Terminal,
}
impl ToolProfile {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Inspect => "inspect",
            Self::Coding => "coding",
            Self::Terminal => "terminal",
        }
    }
    fn includes(self, name: &str) -> bool {
        match self {
            Self::All => true,
            Self::Inspect => matches!(name, "list" | "read" | "search" | "remember"),
            Self::Coding => name != "terminal",
            Self::Terminal => matches!(name, "list" | "read" | "remember" | "terminal"),
        }
    }
}
pub fn focused_tools(profile: ToolProfile) -> Value {
    let mut list = tool_list();
    list["tools"]
        .as_array_mut()
        .expect("tool schema")
        .retain(|t| profile.includes(t["name"].as_str().unwrap_or("")));
    list
}

/// Kept separate from retrieved memory so trimming history cannot remove the
/// actual capability contract or send a small model after unavailable tools.
pub fn guidance(profile: ToolProfile, policy: Policy, checks: &[project::CheckSpec]) -> String {
    let names = focused_tools(profile)["tools"]
        .as_array()
        .expect("tool schema")
        .iter()
        .filter_map(|v| v["name"].as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let mut text = format!(
        "Available actions\nTool focus: {}. Tools: {names}. Access: {}.\n",
        profile.label(),
        policy.label()
    );
    if profile.includes("run_check") {
        let names = checks
            .iter()
            .take(32)
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>();
        text.push_str(&format!("Configured run_check names: {}. Use an exact name; never pass a shell command as its name.\n", serde_json::to_string(&names).expect("check names")));
        if checks.is_empty() {
            text.push_str("No named checks are configured. The user can select Prepare project checks on Home or Configure checks in Task. Do not invent check names.\n");
        }
    }
    if profile.includes("terminal") && policy == Policy::Trusted {
        text.push_str("terminal can execute arbitrary commands, networking and external tools. Use it for the project's actual tests when no named check covers the task. list supplies the Alt executable and data directory for packs/jobs/task commands; read their --help first.\n");
    } else {
        text.push_str("Terminal execution is unavailable in this selection. Do not request terminal or invent another shell tool. ");
        if profile.includes("run_check") && !checks.is_empty() && policy != Policy::ReviewOnly {
            text.push_str("Execute tests through the configured run_check names above.\n");
        } else {
            text.push_str("Explain any execution gap to the user; verification remains unperformed. The user can configure checks or explicitly select All/Terminal focus and Full access.\n");
        }
    }
    text.push_str("After a failed check, read its actual output, repair the implementation and rerun the check. Report unresolved failures. A claimed or printed edit is not an applied edit.\n");
    text
}

#[derive(Debug, Clone, Default)]
struct ContextState {
    task: String,
    session: String,
}
type Decisions = Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>;
pub struct Bridge {
    pub events: mpsc::Receiver<Event>,
    pub path: PathBuf,
    decisions: Decisions,
    context: Arc<Mutex<ContextState>>,
    pub cancel: Arc<AtomicBool>,
    listener: tokio::task::JoinHandle<()>,
    _directory: tempfile::TempDir,
    pub data: PathBuf,
    pub cwd: PathBuf,
    pub policy: Policy,
    pub tool_profile: ToolProfile,
}
#[derive(Clone)]
struct Service {
    data: PathBuf,
    cwd: PathBuf,
    policy: Policy,
    tool_profile: ToolProfile,
    events: mpsc::Sender<Event>,
    decisions: Decisions,
    context: Arc<Mutex<ContextState>>,
    cancel: Arc<AtomicBool>,
    extensions: Arc<crate::extensions::Manager>,
}
impl Bridge {
    pub fn start(data: &Path, cwd: &Path, policy: Policy) -> Result<Self> {
        let directory = tempfile::Builder::new().prefix("alt-ipc-").tempdir()?;
        crate::config::private_dir(directory.path())?;
        let path = directory.path().join("tools.sock");
        let socket = UnixListener::bind(&path)?;
        let (events_tx, events) = mpsc::channel(64);
        let decisions: Decisions = Arc::default();
        let context = Arc::new(Mutex::new(ContextState::default()));
        let cancel = Arc::new(AtomicBool::new(false));
        let service = Service {
            data: data.into(),
            cwd: cwd.into(),
            policy,
            tool_profile: crate::config::Preferences::load(data)?.tool_profile,
            events: events_tx,
            decisions: decisions.clone(),
            context: context.clone(),
            cancel: cancel.clone(),
            extensions: Arc::default(),
        };
        let listener = tokio::spawn(async move {
            // One tool at a time keeps ordering and checkpoint ownership deterministic.
            while let Ok((stream, _)) = socket.accept().await {
                if let Err(e) = serve_connection(stream, &service).await {
                    let _ = service
                        .events
                        .send(Event::Log(format!("Alt tool connection: {e:#}")))
                        .await;
                }
            }
        });
        Ok(Self {
            events,
            path,
            decisions,
            context,
            cancel,
            listener,
            _directory: directory,
            data: data.into(),
            cwd: cwd.into(),
            policy,
            tool_profile: crate::config::Preferences::load(data)?.tool_profile,
        })
    }
    pub fn context(&self, task: &str, session: &str) {
        *self.context.lock().expect("tool context") = ContextState {
            task: task.into(),
            session: session.into(),
        };
        self.cancel.store(false, Ordering::Relaxed);
    }
    pub fn decision(&self, id: &str, allow: bool) -> bool {
        if let Some(tx) = self.decisions.lock().expect("tool decisions").remove(id) {
            let _ = tx.send(allow);
            true
        } else {
            false
        }
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.decisions.lock().expect("tool decisions").clear();
    }
    pub fn extensions(&self) -> Result<Value> {
        let mut values = json!([{"type":"mcp","server":{"name":"alt","command":std::env::current_exe().unwrap_or_else(|_|PathBuf::from("alt")),"args":["tool-server","--socket",self.path,"--tool-profile",self.tool_profile.label()],"env":[]},"timeout":900}]);
        if self.policy == Policy::Trusted {
            for c in crate::extensions::list(&self.data)?
                .into_iter()
                .filter(|c| c.enabled)
            {
                crate::extensions::selected_inventory(&self.data, &c.name)?;
                values.as_array_mut().expect("array").push(json!({"type":"mcp","server":{"name":c.name,"command":std::env::current_exe()?,"args":["--data-dir",self.data,"external-server","--name",c.name,"--socket",self.path],"env":[]},"timeout":180}));
            }
        }
        Ok(values)
    }
}
impl Drop for Bridge {
    fn drop(&mut self) {
        self.cancel();
        self.listener.abort();
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    name: String,
    arguments: Value,
}
async fn serve_connection(stream: UnixStream, s: &Service) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut buf = Vec::new();
    (&mut reader)
        .take(1024 * 1024 + 1)
        .read_until(b'\n', &mut buf)
        .await?;
    ensure!(buf.len() <= 1024 * 1024, "Tool request exceeds 1 MiB");
    let request: Request = serde_json::from_slice(&buf)?;
    let context = s.context.lock().expect("context").clone();
    let signature = project::digest(&serde_json::to_vec(&request)?);
    let id = format!("alt:{}", uuid::Uuid::new_v4());
    let result = execute(s, &context, &id, &request, &signature).await;
    let failed = result.as_ref().map(tool_failed).unwrap_or(true);
    if let Ok(p) = Project::open(&s.data, &s.cwd) {
        let _ = p.attempt(&context.task, &signature, Some(failed));
    }
    let value = match result {
        Ok(v) if request.name == "extension" => v,
        Ok(v) => {
            // Plain excerpts avoid forcing small models to copy JSON-escaped
            // newlines out of a tool result. Structured evidence remains in SQLite.
            let text = match request.name.as_str() {
                "read" => format!(
                    "File: {} · read version: {} · {} lines\n{}",
                    v["path"].as_str().unwrap_or(""),
                    v["sha256"].as_str().unwrap_or(""),
                    v["total_lines"],
                    v["text"].as_str().unwrap_or("")
                ),
                "run_check" | "terminal" => format!(
                    "Evidence ID: {}\nExit code: {} · timeout: {} · cancelled: {}\n{}\n{}\n{}",
                    v["id"].as_str().unwrap_or(""),
                    v["exit_code"],
                    v["timed_out"],
                    v["cancelled"],
                    v["error"].as_str().unwrap_or(""),
                    v["output"].as_str().unwrap_or(""),
                    v["next_action"].as_str().unwrap_or("")
                ),
                _ => serde_json::to_string(&v)?,
            };
            let preview = if text.chars().count() > 48_000 {
                format!(
                    "{}\n[Preview shortened. Task evidence/export retains the captured output.]",
                    project::bounded(&text, 48_000)
                )
            } else {
                text
            };
            json!({"content":[{"type":"text","text":preview}],"isError":failed})
        }
        Err(e) => json!({"content":[{"type":"text","text":format!("{e:#}")}],"isError":true}),
    };
    let mut bytes = serde_json::to_vec(&value)?;
    bytes.push(b'\n');
    writer.write_all(&bytes).await?;
    Ok(())
}

fn tool_failed(v: &Value) -> bool {
    v["isError"] == true
        || v["error"].is_string()
        || v["timed_out"] == true
        || v["cancelled"] == true
        || (v.get("exit_code").is_some() && v["exit_code"] != 0)
}

#[cfg(test)]
mod evidence_tests {
    use super::*;
    #[test]
    fn invalid_evidence_is_a_tool_failure_even_when_command_exits_zero() {
        assert!(tool_failed(
            &json!({"exit_code":0,"error":"Missing structured report"})
        ));
        assert!(tool_failed(&json!({"exit_code":0,"timed_out":true})));
        assert!(tool_failed(&json!({"exit_code":0,"cancelled":true})));
        assert!(tool_failed(&json!({"exit_code":null})));
        assert!(!tool_failed(&json!({"exit_code":0,"error":null})));
        assert!(!tool_failed(&json!({"checkpoint":"applied"})));
    }
}
async fn approve(
    s: &Service,
    c: &ContextState,
    id: &str,
    title: &str,
    input: &Value,
    detail: &str,
) -> Result<()> {
    ensure!(!s.cancel.load(Ordering::Relaxed), "Task was stopped");
    let (tx, rx) = oneshot::channel();
    s.decisions
        .lock()
        .expect("tool decisions")
        .insert(id.into(), tx);
    let params = json!({"sessionId":c.session,"toolCall":{"toolCallId":id,"title":title,"kind":"other","rawInput":input,"content":[{"type":"content","content":{"type":"text","text":detail}}]},"options":[{"optionId":"reject","name":"Reject","kind":"reject_once"},{"optionId":"allow","name":"Allow once","kind":"allow_once"}]});
    s.events
        .send(Event::Permission {
            id: json!(id),
            params,
        })
        .await?;
    let allowed = tokio::time::timeout(Duration::from_secs(600), rx)
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or(false);
    s.decisions.lock().expect("tool decisions").remove(id);
    ensure!(
        allowed && !s.cancel.load(Ordering::Relaxed),
        "Action rejected or cancelled; no requested action was executed"
    );
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadArgs {
    path: String,
    #[serde(default = "one")]
    start_line: usize,
    #[serde(default = "hundred")]
    lines: usize,
}
fn one() -> usize {
    1
}
fn hundred() -> usize {
    100
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchArgs {
    query: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EditArgs {
    path: String,
    #[serde(default)]
    expected_sha256: Option<String>,
    #[serde(default)]
    old_text: String,
    #[serde(default)]
    new_text: String,
    operation: String,
    reason: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoteArgs {
    kind: String,
    text: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckArgs {
    name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TerminalArgs {
    command: String,
    #[serde(default = "terminal_timeout")]
    timeout_secs: u64,
}
fn terminal_timeout() -> u64 {
    120
}
async fn execute(
    s: &Service,
    c: &ContextState,
    id: &str,
    r: &Request,
    signature: &str,
) -> Result<Value> {
    ensure!(!c.task.is_empty(), "No active task");
    ensure!(!s.cancel.load(Ordering::Relaxed), "Task was stopped");
    Project::open(&s.data, &s.cwd)?.attempt(&c.task, signature, None)?;
    match r.name.as_str() {
        "extension" => {
            ensure!(
                s.policy == Policy::Trusted,
                "External extensions require Full access"
            );
            let name = r.arguments["connection"]
                .as_str()
                .context("Missing connection")?;
            let tool = r.arguments["tool"].as_str().context("Missing tool name")?;
            approve(s,c,id,&format!("External tool · {name}/{tool}"),&r.arguments,"Runs the selected external server tool with its configured host/network access. External side effects cannot be undone.").await?;
            s.extensions
                .call(&s.data, name, tool, r.arguments["arguments"].clone())
                .await
        }
        "list" => {
            ensure!(r.arguments == json!({}), "list takes no arguments");
            let p = Project::open(&s.data, &s.cwd)?;
            let files = p.browse()?;
            Ok(
                json!({"files":files.iter().take(256).collect::<Vec<_>>(),"total_files":files.len(),"files_omitted":files.len().saturating_sub(256),"checks":p.checks()?,"verification":p.verification()?,"access":s.policy.label(),"alt_cli":std::env::current_exe()?,"data_directory":s.data,"workflow_help":"Use search for files beyond this bounded listing. Full access: use alt_cli --data-dir data_directory packs --help or jobs --help for structured workflows and persistent terminals. Quote paths as shell arguments. Call --help for exact options before a new command."}),
            )
        }
        "read" => {
            let a: ReadArgs = serde_json::from_value(r.arguments.clone())?;
            Project::open(&s.data, &s.cwd)?.read(&c.task, &a.path, a.start_line, a.lines)
        }
        "search" => {
            let a: SearchArgs = serde_json::from_value(r.arguments.clone())?;
            Ok(json!(Project::open(&s.data, &s.cwd)?.search(&a.query)?))
        }
        "remember" => {
            let a: NoteArgs = serde_json::from_value(r.arguments.clone())?;
            ensure!(
                matches!(a.kind.as_str(), "plan" | "decision" | "next"),
                "Model memory supports plan, decision, next; verification comes from actual tool results"
            );
            Project::open(&s.data, &s.cwd)?.note(
                &c.task,
                &a.kind,
                &a.text,
                "model note (unverified)",
            )?;
            Ok(json!({"saved":true,"verified":false}))
        }
        "edit" => {
            ensure!(
                s.policy != Policy::ReviewOnly,
                "Review only is selected; propose changes in your response or ask the user to choose Guided changes"
            );
            let a: EditArgs = serde_json::from_value(r.arguments.clone())?;
            ensure!(
                a.operation == "delete" || r.arguments.get("new_text").is_some(),
                "new_text is required for create/replace. Send old_text and new_text as separate native tool arguments, not a JSON example inside a string."
            );
            let change = {
                Project::open(&s.data, &s.cwd)?.prepare_edit(
                    &c.task,
                    &a.path,
                    a.expected_sha256.as_deref(),
                    &a.old_text,
                    &a.new_text,
                    &a.operation,
                    &a.reason,
                )?
            };
            let detail = format!(
                "{}\n{}\n{}\nCheckpoint: {}\n{}",
                a.reason,
                s.policy.label(),
                if change.test_change {
                    "TEST/ASSERTION CHANGE: passing changed checks does not prove the original bug is fixed."
                } else {
                    "Original file will be saved for undo."
                },
                change.id,
                change.diff()
            );
            if let Err(e) = approve(
                s,
                c,
                id,
                &format!("{} {}", a.operation, a.path),
                &r.arguments,
                &detail,
            )
            .await
            {
                Project::open(&s.data, &s.cwd)?.reject(&change.id)?;
                return Err(e);
            }
            let applied = Project::open(&s.data, &s.cwd)?.apply(&change.id, s.policy)?;
            let p = Project::open(&s.data, &s.cwd)?;
            let syntax = p.bytes(&applied.path)?.and_then(|bytes| String::from_utf8(bytes).ok())
                .map(|body| crate::syntax::chunks(&applied.path, &body)).transpose()?
                .map(|syntax| json!({"parser":syntax.parser,"contains_parse_errors":syntax.contains_parse_errors,"scope":"Syntax only; run independent behavioral checks"}));
            Ok(
                json!({"checkpoint":applied.id,"path":applied.path,"sha256":applied.after_hash,"syntax":syntax,"configured_checks":p.checks()?.iter().map(|c|&c.name).collect::<Vec<_>>(),"next":"Inspect any syntax errors, then run a fresh configured check. File changed; previous results are stale. Do not invent a check name."}),
            )
        }
        "run_check" => {
            let a: CheckArgs = serde_json::from_value(r.arguments.clone())?;
            let checks = Project::open(&s.data, &s.cwd)?.checks()?;
            let check = checks.iter().find(|v| v.name == a.name).with_context(|| {
                format!(
                    "No configured check named {:?}. {}",
                    a.name,
                    guidance(s.tool_profile, s.policy, &checks)
                )
            })?;
            ensure!(
                s.policy != Policy::ReviewOnly,
                "Review only does not execute code"
            );
            approve(s,c,id,&format!("Run {}",a.name),&json!(check),&format!("{}\nRuns on a disposable copy of the project. The result records the exact source snapshot. Dependencies must already be available.",s.policy.label())).await?;
            let result = sandbox::run(
                s.data.clone(),
                s.cwd.clone(),
                c.task.clone(),
                a.name,
                s.policy,
                s.cancel.clone(),
            )
            .await?;
            let mut value = serde_json::to_value(&result)?;
            if result.exit_code != Some(0) || result.error.is_some() {
                let changes = Project::open(&s.data, &s.cwd)?.task(&c.task)?.changes;
                value["next_action"] = json!(if changes == 0 {
                    "No tracked edit has been applied. Invoke edit as a NATIVE TOOL CALL. Printing a JSON code block does not change a file. Read the file, then call edit with path, operation=replace, old_text, new_text, reason. Alt remembers the read hash; expected_sha256 is optional."
                } else {
                    "Inspect the actual failed output and current files; adjust the implementation before repeating this check."
                });
            }
            Ok(value)
        }
        "terminal" => {
            let a: TerminalArgs = serde_json::from_value(r.arguments.clone())?;
            ensure!(
                s.policy == Policy::Trusted,
                "Terminal requires Full access (Trusted commands) in Settings. Guided changes uses reviewed edits and isolated checks."
            );
            ensure!(
                !a.command.trim().is_empty()
                    && a.command.len() <= 16000
                    && (1..=600).contains(&a.timeout_secs),
                "Use a command up to 16 KB and a 1–600 second timeout"
            );
            approve(s,c,id,"Terminal · FULL HOST ACCESS",&r.arguments,"Runs in the real project with your OS permissions, environment, network and external tools. External side effects cannot be undone. Stop terminates owned processes; it does not reverse completed actions.").await?;
            let result = sandbox::terminal(
                &s.data,
                &s.cwd,
                &c.task,
                &a.command,
                a.timeout_secs,
                s.cancel.clone(),
            )
            .await?;
            Ok(json!(result))
        }
        _ => bail!("Unknown Alt tool: {}", r.name),
    }
}
pub fn tool_list() -> Value {
    let tool = |name: &str, description: &str, props: Value, required: Vec<&str>| json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":props,"required":required,"additionalProperties":false},"annotations":{"readOnlyHint":true,"openWorldHint":false}});
    json!({"tools":[
        tool("list","List project files, configured checks, and active access mode. Excludes generated folders, secrets and links.",json!({}),vec![]),
        tool("read","Read a current file; Alt remembers its content hash. Required before edit; use exact old_text without line-number prefixes.",json!({"path":{"type":"string"},"start_line":{"type":"integer","minimum":1},"lines":{"type":"integer","minimum":1,"maximum":300}}),vec!["path"]),
        tool("search","Search current project text and symbols with lexical retrieval. Excerpts are not an edit authorization.",json!({"query":{"type":"string"}}),vec!["query"]),
        tool("remember","Persist a short plan before editing, a decision, or next step. Notes never certify success.",json!({"kind":{"type":"string","enum":["plan","decision","next"]},"text":{"type":"string"}}),vec!["kind","text"]),
        tool("edit","Review and apply one focused change with an undo checkpoint. Read existing files first. Alt asks the user before applying; never bypass a rejection.",json!({"path":{"type":"string"},"old_text":{"type":"string","description":"Exact unique substring of the current file. Prefer a short single-line substring without line numbers."},"new_text":{"type":"string","description":"Replacement text. Must be a separate argument from old_text."},"operation":{"type":"string","enum":["replace","create","delete"]},"reason":{"type":"string"}}),vec!["path","operation","reason","old_text","new_text"]),
        tool("run_check","Run a user-configured named check after approval. list shows names; argv cannot be set by this tool.",json!({"name":{"type":"string"}}),vec!["name"]),
        tool("terminal","Arbitrary shell command in the real project, including networking, installs and external tools. Only available in user-selected Full access mode. External side effects are not undoable.",json!({"command":{"type":"string"},"timeout_secs":{"type":"integer","minimum":1,"maximum":600}}),vec!["command"])
    ]})
}
/// Private stdio transport launched only by the selected ACP engine.
pub async fn stdio(socket: &Path, profile: ToolProfile) -> Result<()> {
    let mut input = BufReader::new(tokio::io::stdin());
    let mut output = tokio::io::stdout();
    loop {
        let mut bytes = Vec::new();
        let n = (&mut input)
            .take(1024 * 1024 + 1)
            .read_until(b'\n', &mut bytes)
            .await?;
        if n == 0 {
            return Ok(());
        }
        ensure!(n <= 1024 * 1024, "MCP frame exceeds 1 MiB");
        let v: Value = serde_json::from_slice(&bytes)?;
        let Some(id) = v.get("id") else {
            continue;
        };
        let result: Result<Value> = match v["method"].as_str().unwrap_or("") {
            "initialize" => Ok(
                json!({"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"Alt controlled tools","version":env!("CARGO_PKG_VERSION")}}),
            ),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(focused_tools(profile)),
            "tools/call" => {
                async {
                    let name = v["params"]["name"].as_str().context("Missing tool name")?;
                    let mut stream = UnixStream::connect(socket)
                        .await
                        .context("Alt workspace disconnected")?;
                    let request = Request {
                        name: name.into(),
                        arguments: v["params"].get("arguments").cloned().unwrap_or(json!({})),
                    };
                    let mut bytes = serde_json::to_vec(&request)?;
                    bytes.push(b'\n');
                    stream.write_all(&bytes).await?;
                    let mut response = Vec::new();
                    BufReader::new(stream)
                        .take(1024 * 1024 + 1)
                        .read_until(b'\n', &mut response)
                        .await?;
                    ensure!(response.len() <= 1024 * 1024, "Tool output exceeded limit");
                    Ok(serde_json::from_slice(&response)?)
                }
                .await
            }
            _ => Err(anyhow::anyhow!("Unsupported MCP method")),
        };
        let response = match result {
            Ok(r) => json!({"jsonrpc":"2.0","id":id,"result":r}),
            Err(e) => {
                json!({"jsonrpc":"2.0","id":id,"error":{"code":-32602,"message":e.to_string()}})
            }
        };
        let mut bytes = serde_json::to_vec(&response)?;
        bytes.push(b'\n');
        output.write_all(&bytes).await?;
        output.flush().await?;
    }
}
