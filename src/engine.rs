//! Minimal ACP v1 client. All engine-specific process configuration stays here.
use crate::config::{Profile, Provider};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    path::Path,
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command},
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

pub type Response = oneshot::Receiver<std::result::Result<Value, String>>;
type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<std::result::Result<Value, String>>>>>;
pub const MAX_FRAME: usize = 1024 * 1024;

#[derive(Debug)]
pub enum Event {
    Update(Value),
    Permission { id: Value, params: Value },
    Log(String),
    Disconnected(String),
}

pub struct Engine {
    child: Child,
    group: crate::process::OwnedGroup,
    stdin: ChildStdin,
    pending: Pending,
    events: mpsc::Receiver<Event>,
    tasks: Vec<JoinHandle<()>>,
    next_id: u64,
    bridge: Option<crate::toolbox::Bridge>,
    active_session: Option<String>,
    logical_session: Option<String>,
    task: String,
    context_tokens: u32,
    local_tokenizer: Option<Profile>,
    relay: Option<crate::inference::Relay>,
    instruction_draft: Option<String>,
}

/// Read one newline-delimited frame without allowing unbounded allocation.
async fn frame<R: AsyncBufRead + Unpin>(reader: &mut R) -> Result<Option<Vec<u8>>> {
    let mut out = Vec::new();
    loop {
        let bytes = reader.fill_buf().await?;
        if bytes.is_empty() {
            if out.is_empty() {
                return Ok(None);
            }
            bail!("Engine closed mid-frame");
        }
        let end = bytes.iter().position(|b| *b == b'\n').map(|i| i + 1);
        let count = end.unwrap_or(bytes.len());
        ensure!(out.len() + count <= MAX_FRAME, "Engine frame exceeds 1 MiB");
        out.extend_from_slice(&bytes[..count]);
        reader.consume(count);
        if end.is_some() {
            return Ok(Some(out));
        }
    }
}

/// Goose 1.53 converts provider transport and allowance failures into an engine-generated
/// message followed by end_turn. Recognize its exact synthetic-message shape;
/// ordinary model text and tool errors must not be classified by keywords.
fn goose_provider_error(params: &Value) -> Option<String> {
    let update = &params["update"];
    if update["sessionUpdate"] != "agent_message_chunk"
        || !update["_meta"]["goose"]["messageId"]
            .as_str()?
            .starts_with("msg_")
    {
        return None;
    }
    let text = update["content"]["text"].as_str()?;
    if text.starts_with("Network error: ")
        && text.ends_with("\n\nPlease resend your message to try again.")
    {
        Some(format!(
            "Model connection interrupted. Your conversation was saved; reconnect and resend. {}",
            text.trim_end_matches("\n\nPlease resend your message to try again.")
        ))
    } else if let Some(issue) = text.strip_prefix("Ran into this error: ").and_then(|s| {
        s.strip_suffix("\n\nPlease retry if you think this is a transient or recoverable error.")
    }) {
        if issue
            == "Rate limit exceeded: Selected shared effort allowance exhausted; no extra model call was sent."
        {
            Some("Model call allowance exhausted. Your conversation and source were saved. Choose Allowance to add a finite allowance to this connection, or adjust Model effort and reconnect. A new message alone does not reset the allowance.".into())
        } else {
            Some(format!(
                "Model request failed. Your conversation and source were saved. {issue}"
            ))
        }
    } else {
        None
    }
}

impl Engine {
    pub async fn goose(binary: &Path, root: &Path, cwd: &Path, profile: &Profile) -> Result<Self> {
        Self::goose_with_policy(
            binary,
            root,
            cwd,
            profile,
            crate::config::Preferences::load(root)?.access_policy,
        )
        .await
    }

    pub async fn goose_with_policy(
        binary: &Path,
        root: &Path,
        cwd: &Path,
        profile: &Profile,
        policy: crate::project::Policy,
    ) -> Result<Self> {
        profile.validate()?;
        let bridge = crate::toolbox::Bridge::start(root, cwd, policy)?;
        let relay = crate::inference::Relay::start(root, profile).await?;
        let goose_root = root.join("engine-v3");
        crate::config::private_dir(&goose_root)?;
        let goose_root = goose_root.canonicalize()?;
        crate::config::private_dir(&goose_root.join("config"))?;
        crate::config::atomic_write(&goose_root.join("config/config.yaml"),b"extensions:\n  developer:\n    enabled: false\n    type: platform\n    name: developer\n    description: Disabled by Alt; Alt owns execution\n")?;
        let mut command = Command::new(binary);
        // Goose always talks to Alt's loopback relay, including external model
        // profiles. Keep its per-connection capability out of inherited proxies.
        let mut no_proxy = std::env::var("NO_PROXY").unwrap_or_default();
        if let Ok(lower) = std::env::var("no_proxy")
            && !lower.is_empty()
        {
            no_proxy.push(',');
            no_proxy.push_str(&lower);
        }
        no_proxy.push_str(",127.0.0.1,localhost,::1");
        command
            .arg("acp")
            .current_dir(cwd)
            .env("GOOSE_PATH_ROOT", goose_root)
            .env("GOOSE_MODEL", &profile.model)
            .env("GOOSE_MODE", "auto")
            .env("NO_PROXY", &no_proxy)
            .env("no_proxy", no_proxy)
            .env_remove("GOOSE_ADDITIONAL_CONFIG_FILES")
            .env("GOOSE_MAX_TURNS", profile.max_turns.to_string())
            .env(
                "GOOSE_MAX_TOKENS",
                profile.effective_inference().output_tokens.to_string(),
            )
            .env("GOOSE_DISABLE_SESSION_NAMING", "true")
            .env("GOOSE_CONTEXT_LIMIT", profile.context_tokens.to_string())
            .env(
                "GOOSE_INPUT_LIMIT",
                (profile.context_tokens - profile.effective_inference().output_tokens).to_string(),
            )
            .env("GOOSE_TOOLSHIM", "false")
            .env_remove("GOOSE_MOIM_MESSAGE_TEXT")
            .env_remove("GOOSE_MOIM_MESSAGE_FILE")
            .env("GOOSE_TELEMETRY_ENABLED", "false")
            .env_remove("GOOSE_LEAD_MODEL")
            .env_remove("GOOSE_LEAD_PROVIDER")
            .env_remove("OPENAI_HOST")
            .env_remove("OPENAI_BASE_PATH")
            .env_remove("OPENAI_API_KEY")
            .env_remove("OPENAI_ORGANIZATION")
            .env_remove("OPENAI_PROJECT")
            .env_remove("OPENAI_CUSTOM_HEADERS");
        // Explicit endpoint and credentials prevent inherited OpenAI settings from
        // silently routing a local profile to a different account/server.
        match profile.provider {
            Provider::Openai => {
                command
                    .env("GOOSE_PROVIDER", "openai")
                    .env("OPENAI_BASE_URL", &relay.endpoint);
            }
            Provider::Ollama => {
                // Goose 1.53's Ollama provider sends options.num_predict to /v1,
                // but Ollama 0.35's OpenAI handler only accepts top-level max_tokens.
                // Use the standard compatible client for chat, retaining native
                // Ollama inventory/evaluation elsewhere and the exact selected tag.
                // Native context remains a server/tag setting, unlike Alt's budget.
                command
                    .env("GOOSE_PROVIDER", "openai")
                    .env("OPENAI_BASE_URL", &relay.endpoint);
            }
        }
        let mut engine = Self::spawn(command).await?;
        engine.bridge = Some(bridge);
        engine.context_tokens = profile.context_tokens;
        engine.local_tokenizer = profile.local_model.as_ref().map(|_| profile.clone());
        engine.relay = Some(relay);
        Ok(engine)
    }

    /// Public for alternate ACP engines and deterministic protocol fixtures.
    pub async fn spawn(mut command: Command) -> Result<Self> {
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        crate::process::configure(&mut command);
        let mut child = command
            .spawn()
            .context("Cannot launch engine; install Goose 1.53.0 or pass --engine PATH")?;
        let group = crate::process::OwnedGroup::capture(&child);
        let stdin = child.stdin.take().context("Missing engine stdin")?;
        let stdout = child.stdout.take().context("Missing engine stdout")?;
        let stderr = child.stderr.take().context("Missing engine stderr")?;
        let pending: Pending = Arc::default();
        let (tx, events) = mpsc::channel(128);
        let responses = pending.clone();
        let events_tx = tx.clone();
        let reader = tokio::spawn(async move {
            let result: Result<()> = async {
                let mut reader = BufReader::new(stdout);
                let mut provider_error = None;
                while let Some(line) = frame(&mut reader).await? {
                    let message: Value =
                        serde_json::from_slice(&line).context("Malformed ACP JSON")?;
                    ensure!(message["jsonrpc"] == "2.0", "Invalid ACP JSON-RPC envelope");
                    if let Some(method) = message["method"].as_str() {
                        let event = match method {
                            "session/update" if message.get("id").is_none() => {
                                if let Some(error) = goose_provider_error(&message["params"]) {
                                    provider_error = Some(error);
                                }
                                Event::Update(message["params"].clone())
                            }
                            "session/request_permission" if message.get("id").is_some() => {
                                Event::Permission {
                                    id: message["id"].clone(),
                                    params: message["params"].clone(),
                                }
                            }
                            _ if message.get("id").is_some() => {
                                // Unknown client requests cannot be silently ignored. The
                                // UI sends MethodNotFound, avoiding hung engine requests.
                                Event::Permission {
                                    id: message["id"].clone(),
                                    params: json!({"unsupportedMethod":method}),
                                }
                            }
                            _ => continue,
                        };
                        if events_tx.send(event).await.is_err() {
                            break;
                        }
                    } else if let Some(id) = message["id"].as_u64() {
                        let sender = responses.lock().expect("pending mutex").remove(&id);
                        if let Some(sender) = sender {
                            let result = if let Some(error) = provider_error.take() {
                                Err(error)
                            } else if let Some(error) = message.get("error") {
                                Err(error.to_string())
                            } else if let Some(result) = message.get("result") {
                                Ok(result.clone())
                            } else {
                                Err("ACP response missing result/error".into())
                            };
                            let _ = sender.send(result);
                        }
                    } else {
                        bail!("Invalid ACP response ID");
                    }
                }
                Ok(())
            }
            .await;
            responses.lock().expect("pending mutex").clear();
            let message = result
                .err()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "Engine exited".into());
            let _ = events_tx.send(Event::Disconnected(message)).await;
        });
        let logs = tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            while let Ok(Some(line)) = frame(&mut reader).await {
                // Logs are diagnostic, never allowed to block the protocol.
                let text = String::from_utf8_lossy(&line).chars().take(4096).collect();
                let _ = tx.try_send(Event::Log(text));
            }
        });
        Ok(Self {
            child,
            group,
            stdin,
            pending,
            events,
            tasks: vec![reader, logs],
            next_id: 1,
            bridge: None,
            active_session: None,
            logical_session: None,
            task: String::new(),
            context_tokens: 8192,
            local_tokenizer: None,
            relay: None,
            instruction_draft: None,
        })
    }

    async fn send(&mut self, message: Value) -> Result<()> {
        let mut bytes = serde_json::to_vec(&message)?;
        ensure!(bytes.len() < MAX_FRAME, "Request exceeds 1 MiB");
        bytes.push(b'\n');
        tokio::time::timeout(Duration::from_secs(5), self.stdin.write_all(&bytes))
            .await
            .context("Engine stdin stalled")??;
        Ok(())
    }
    pub fn set_instruction_draft(&mut self, body: String) -> Result<()> {
        ensure!(
            !body.trim().is_empty() && body.len() <= 5000,
            "Instruction trial must be nonempty and at most 5000 UTF-8 bytes"
        );
        self.instruction_draft = Some(body);
        Ok(())
    }

    pub async fn request(&mut self, method: &str, params: Value) -> Result<Response> {
        let id = self.next_id;
        self.next_id += 1;
        let (tx, rx) = oneshot::channel();
        self.pending.lock().expect("pending mutex").insert(id, tx);
        if let Err(error) = self
            .send(json!({"jsonrpc":"2.0", "id":id, "method":method, "params":params}))
            .await
        {
            self.pending.lock().expect("pending mutex").remove(&id);
            return Err(error);
        }
        Ok(rx)
    }

    /// Setup calls drain replay notifications; persisted Alt events render history.
    pub async fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        let mut response = self.request(method, params).await?;
        let timeout = tokio::time::sleep(Duration::from_secs(45));
        tokio::pin!(timeout);
        loop {
            tokio::select! {
                result = &mut response => {
                    // Notifications and responses use separate channels. Drain
                    // replay that arrived before this response so loading a
                    // session does not duplicate its persisted transcript.
                    while let Some(event) = self.try_event() {
                        match event {
                            Event::Permission { id, params } => self.permission(id, &params, false).await?,
                            Event::Disconnected(reason) => bail!("{reason}"),
                            _ => {},
                        }
                    }
                    return response_value(result);
                },
                _ = &mut timeout => bail!("Engine timed out during {method}"),
                event = self.next_event() => match event {
                    Some(Event::Permission {id, params}) => self.permission(id, &params, false).await?,
                    Some(Event::Disconnected(reason)) => bail!("{reason}"),
                    None => bail!("Engine event stream closed"),
                    _ => {},
                }
            }
        }
    }

    pub async fn initialize(&mut self) -> Result<Value> {
        let value = self.call("initialize", json!({"protocolVersion":1,
            "clientCapabilities":{"fs":{"readTextFile":false,"writeTextFile":false},"terminal":false},
            "clientInfo":{"name":"alt-cli","version":env!("CARGO_PKG_VERSION")}})).await?;
        ensure!(
            value["protocolVersion"] == 1,
            "Engine did not negotiate ACP v1"
        );
        Ok(value)
    }

    pub async fn next_event(&mut self) -> Option<Event> {
        let event = if let Some(bridge) = &mut self.bridge {
            tokio::select! { event=self.events.recv()=>event,event=bridge.events.recv()=>event }
        } else {
            self.events.recv().await
        };
        event.map(|event| self.normalize(event))
    }

    pub async fn new_session(&mut self, cwd: &Path) -> Result<Value> {
        if self.bridge.is_some()
            && let Some(previous) = self.active_session.take()
        {
            self.call("session/close", json!({"sessionId":previous}))
                .await?;
        }
        let extensions = self
            .bridge
            .as_ref()
            .map(|b| b.extensions())
            .transpose()?
            .unwrap_or(json!([]));
        let result = self
            .call(
                "session/new",
                json!({"cwd":cwd,"mcpServers":[],"_meta":{"enabledExtensions":extensions}}),
            )
            .await?;
        self.active_session = result["sessionId"].as_str().map(str::to_owned);
        Ok(result)
    }

    pub fn set_task(&mut self, task: &str) {
        self.task = task.into();
    }
    pub fn inference_status(&self) -> Option<crate::inference::Status> {
        self.relay.as_ref().map(crate::inference::Relay::status)
    }
    pub fn add_allowance(&self, requests: u32, tokens: u32) -> Result<crate::inference::Status> {
        self.relay
            .as_ref()
            .context("No active inference relay")?
            .add_allowance(requests, tokens)
    }
    pub fn verification(&self) -> Result<Option<crate::project_services::Verification>> {
        self.bridge
            .as_ref()
            .map(|b| crate::project::Project::open(&b.data, &b.cwd)?.verification())
            .transpose()
    }

    /// Reconstruct a bounded context from Alt's durable ledger. Never reload legacy unrestricted tools.
    pub async fn restore_session(&mut self, cwd: &Path) -> Result<()> {
        self.new_session(cwd).await?;
        Ok(())
    }

    pub fn try_event(&mut self) -> Option<Event> {
        self.bridge
            .as_mut()
            .and_then(|b| b.events.try_recv().ok())
            .or_else(|| self.events.try_recv().ok())
            .map(|e| self.normalize(e))
    }
    fn normalize(&self, mut event: Event) -> Event {
        if let (Some(active), Some(logical)) = (&self.active_session, &self.logical_session) {
            match &mut event {
                Event::Update(params) | Event::Permission { params, .. }
                    if params["sessionId"] == *active =>
                {
                    params["sessionId"] = json!(logical)
                }
                _ => {}
            }
        }
        event
    }

    pub async fn prompt(&mut self, session: &str, text: &str) -> Result<Response> {
        self.prompt_with_brief(session, text, "").await
    }

    pub async fn prompt_with_brief(
        &mut self,
        session: &str,
        text: &str,
        brief: &str,
    ) -> Result<Response> {
        self.logical_session = Some(session.into());
        let mut memory = String::new();
        let mut available_actions = String::new();
        let mut active = session.to_string();
        if let Some(bridge) = &self.bridge {
            let data = bridge.data.clone();
            let cwd = bridge.cwd.clone();
            let task = if self.task.is_empty() {
                session.to_string()
            } else {
                self.task.clone()
            };
            let context = self.context_tokens;
            // UTF-8 bytes are a conservative budget proxy; the engine also enforces its native window.
            ensure!(
                text.len() + brief.len() < context as usize * 2,
                "Message and brief exceed the input budget; shorten them or increase context"
            );
            let task_copy = task.clone();
            let request = text.to_owned();
            let cancel = bridge.cancel.clone();
            cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            let focus = bridge.tool_profile;
            let compact = focus.is_compact();
            let policy = bridge.policy;
            let workflow = crate::config::Preferences::load(&data)?.workflow;
            let active_skill = crate::config::Preferences::load(&data)?.active_skill;
            available_actions =
                crate::project_worker::run(data.clone(), cwd.clone(), cancel.clone(), move |p| {
                    Ok(crate::toolbox::guidance(focus, policy, &p.checks()?))
                })
                .await?;
            if compact && active_skill.is_none() {
                available_actions.push_str("\nUse skill(action=list/read/helper) for optional executable procedures when needed.\n");
            } else {
                available_actions.push_str(&format!(
                    "\n{}",
                    crate::skills::prompt(active_skill.as_deref(), text)?
                ));
            }
            memory = crate::project_worker::run(data.clone(), cwd.clone(), cancel.clone(), move |project| {
                project.start_task(&task_copy, &request)?;
                crate::workflow::begin(project,&task_copy,workflow)?;
                let mut memory = if compact {
                    project.compact_memory(&task_copy, &request, (context as usize / 2).clamp(512,6000))?
                } else { project.memory(&task_copy, &request, (context as usize).min(16000))? };
                if !compact && let Some(check) = project.latest_check(&task_copy)? {
                    memory.push_str(&format!("\nLast check observation (may be stale; use list for current verification): {} exit={:?}, error={:?}, tests={:?}.\n", check.name, check.exit_code, check.error, check.tests_run));
                }
                Ok(memory)
            }).await?;
            if let Some(profile) = &self.local_tokenizer {
                // Counts this memory block with the selected llama.cpp tokenizer.
                // Tool schemas, chat templates and generated replies remain engine-budgeted.
                if let Ok(mut tokens) = crate::benchmark::token_count(profile, &memory).await {
                    let budget = (context as usize / 3).max(256);
                    for _ in 0..3 {
                        if tokens <= budget {
                            break;
                        }
                        let size = (memory.chars().count() * budget / tokens * 9 / 10).max(128);
                        let task_copy = task.clone();
                        let request = text.to_owned();
                        memory = crate::project_worker::run(
                            data.clone(),
                            cwd.clone(),
                            cancel.clone(),
                            move |p| {
                                if compact {
                                    p.compact_memory(&task_copy, &request, size)
                                } else {
                                    p.memory(&task_copy, &request, size)
                                }
                            },
                        )
                        .await?;
                        tokens = crate::benchmark::token_count(profile, &memory)
                            .await
                            .unwrap_or(tokens);
                    }
                    // Preserve complete evidence/excerpt sections instead of cutting through JSON or a file.
                    ensure!(
                        tokens <= budget,
                        "Memory exceeds this model's measured token budget. Shorten pinned requirements or increase context, then retry; the conversation is saved."
                    );
                    let counted = crate::benchmark::token_count(profile, &memory).await.ok();
                    let task_copy = task.clone();
                    let memory_copy = memory.clone();
                    crate::project_worker::run(
                        data.clone(),
                        cwd.clone(),
                        cancel.clone(),
                        move |p| p.record_token_usage(&task_copy, &memory_copy, counted, budget),
                    )
                    .await?;
                }
            }
            // Each turn starts fresh: long-running work lives in Alt memory instead of accumulating unbounded engine history.
            self.new_session(&cwd).await?;
            active = self
                .active_session
                .clone()
                .context("Missing engine session")?;
            self.bridge
                .as_ref()
                .expect("bridge")
                .context(&task, session);
        }
        let operator = self
            .bridge
            .as_ref()
            .map(|b| b.tool_profile)
            .unwrap_or_default()
            .operator();
        let mut prompt = vec![json!({"type":"text","text":operator})];
        if let Some(body) = &self.instruction_draft {
            let id = crate::project::digest(body.as_bytes());
            prompt.push(json!({"type":"text","text":format!("Experimental operator supplement {id} (this invocation only; unpromoted):\n{body}")}));
        } else if let Some(bridge) = &self.bridge
            && let Some(id) = crate::config::Preferences::load(&bridge.data)?.instruction_version
        {
            crate::instructions::validation(&bridge.data, &id)?;
            prompt.push(json!({"type":"text","text":format!("Reviewed operator supplement {id}:\n{}",crate::instructions::text(&bridge.data,&id)?)}));
        }
        if !brief.trim().is_empty() {
            ensure!(brief.len() <= 16000, "The project brief is too large");
            prompt.push(
                json!({"type":"text","text":format!("User-maintained project brief:\n{brief}")}),
            );
        }
        if !memory.is_empty() {
            prompt.push(json!({"type":"text","text":format!("Durable task memory and current evidence (notes are unverified):\n{memory}")}));
        }
        if !available_actions.is_empty() {
            prompt.push(json!({"type":"text","text":available_actions}));
        }
        prompt.push(json!({"type":"text","text":text}));
        self.request(
            "session/prompt",
            json!({"sessionId":active,"prompt":prompt}),
        )
        .await
    }

    pub async fn cancel(&mut self, session: &str) -> Result<()> {
        if let Some(bridge) = &self.bridge {
            bridge.cancel();
        }
        let active = self.active_session.as_deref().unwrap_or(session);
        self.send(json!({"jsonrpc":"2.0","method":"session/cancel","params":{"sessionId":active}}))
            .await
    }

    pub async fn permission(&mut self, id: Value, params: &Value, allow: bool) -> Result<()> {
        if let Some(key) = id.as_str().filter(|v| v.starts_with("alt:")) {
            if let Some(bridge) = &self.bridge {
                bridge.decision(key, allow);
            }
            return Ok(());
        }
        if params.get("unsupportedMethod").is_some() {
            return self.send(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Client method not supported"}})).await;
        }
        let kind = if allow { "allow_once" } else { "reject_once" };
        let selected = params["options"]
            .as_array()
            .and_then(|options| options.iter().find(|option| option["kind"] == kind))
            .and_then(|option| option["optionId"].as_str());
        let outcome = match selected {
            Some(id) => json!({"outcome":"selected","optionId":id}),
            None => json!({"outcome":"cancelled"}),
        };
        self.send(json!({"jsonrpc":"2.0","id":id,"result":{"outcome":outcome}}))
            .await
    }

    pub async fn shutdown(&mut self) {
        self.relay = None;
        if let Some(bridge) = &self.bridge {
            bridge.cancel();
        }
        if self.bridge.is_some()
            && let Some(session) = self.active_session.take()
        {
            let _ = tokio::time::timeout(
                Duration::from_millis(800),
                self.call("session/close", json!({"sessionId":session})),
            )
            .await;
        }
        let _ = self.stdin.shutdown().await;
        self.group.stop(&mut self.child).await;
    }

    fn kill_group(&self) {
        self.group.kill();
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.kill_group();
        for task in &self.tasks {
            task.abort();
        }
    }
}

pub fn response_value(
    result: std::result::Result<std::result::Result<Value, String>, oneshot::error::RecvError>,
) -> Result<Value> {
    result
        .context("Engine disconnected before replying")?
        .map_err(anyhow::Error::msg)
}

pub fn update_text(params: &Value) -> Option<String> {
    let update = &params["update"];
    match update["sessionUpdate"].as_str()? {
        "agent_message_chunk" => update["content"]["text"].as_str().map(str::to_owned),
        "tool_call" | "tool_call_update" => {
            let mut text = format!(
                "\n[tool {}: {} {}]\n",
                update["toolCallId"].as_str().unwrap_or("?"),
                update["title"].as_str().unwrap_or(""),
                update["status"].as_str().unwrap_or("")
            );
            if let Some(items) = update["content"].as_array() {
                for item in items {
                    if let Some(content) = item["content"]["text"].as_str() {
                        text.extend(content.chars().take(16_000));
                        text.push('\n');
                    }
                }
            }
            Some(text)
        }
        _ => None,
    }
}

#[cfg(test)]
mod frame_fuzz_tests {
    #[test]
    fn only_the_pinned_goose_synthetic_network_error_shape_marks_a_failed_turn() {
        use super::*;
        let mut event = json!({"update":{"sessionUpdate":"agent_message_chunk","_meta":{"goose":{"messageId":"msg_failure"}},"content":{"text":"Network error: Stream decode error\n\nPlease resend your message to try again."}}});
        assert!(
            goose_provider_error(&event)
                .unwrap()
                .contains("conversation was saved")
        );
        event["update"]["_meta"]["goose"]["messageId"] = json!("chatcmpl-model-content");
        assert!(goose_provider_error(&event).is_none());
        event["update"]["_meta"]["goose"]["messageId"] = json!("msg_failure");
        event["update"]["content"]["text"] =
            json!("Network error: an example string in your source");
        assert!(goose_provider_error(&event).is_none());
    }
    #[test]
    fn exhausted_shared_allowance_is_a_failed_turn_and_model_prose_is_not() {
        use super::*;
        let text = "Ran into this error: Rate limit exceeded: Selected shared effort allowance exhausted; no extra model call was sent.\n\nPlease retry if you think this is a transient or recoverable error.";
        let mut event = json!({"update":{"sessionUpdate":"agent_message_chunk","_meta":{"goose":{"messageId":"msg_failure"}},"content":{"text":text}}});
        assert!(
            goose_provider_error(&event)
                .unwrap()
                .contains("allowance exhausted")
        );
        event["update"]["_meta"]["goose"]["messageId"] = json!("chatcmpl-model-content");
        assert!(goose_provider_error(&event).is_none());
        event["update"]["_meta"]["goose"]["messageId"] = json!("msg_failure");
        event["update"]["content"]["text"] = json!("Selected shared effort allowance exhausted");
        assert!(goose_provider_error(&event).is_none());
        event["update"]["content"]["text"] = json!(
            "Ran into this error: Request failed with status: 400 Bad Request\n\nPlease retry if you think this is a transient or recoverable error."
        );
        assert!(
            goose_provider_error(&event)
                .unwrap()
                .contains("400 Bad Request")
        );
        event["update"]["_meta"]["goose"]["messageId"] = json!("chatcmpl-model-content");
        assert!(goose_provider_error(&event).is_none());
    }
    #[tokio::test]
    async fn fragmented_and_mutated_acp_frames_obey_size_and_termination_bounds() {
        use super::*;
        for capacity in 1..=32 {
            for size in [0, 1, 31, 1024, 8192] {
                let mut bytes = vec![b'x'; size];
                bytes.push(b'\n');
                bytes.extend_from_slice(b"{}\n");
                let mut reader = tokio::io::BufReader::with_capacity(capacity, bytes.as_slice());
                let first = frame(&mut reader).await.unwrap().unwrap();
                assert_eq!(first.len(), size + 1);
                assert_eq!(frame(&mut reader).await.unwrap().unwrap(), b"{}\n");
                assert!(frame(&mut reader).await.unwrap().is_none());
            }
        }
        let oversized = vec![0; MAX_FRAME + 1];
        assert!(frame(&mut oversized.as_slice()).await.is_err());
        assert!(frame(&mut b"{\"partial\":".as_slice()).await.is_err());
    }
}
