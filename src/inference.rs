//! Owned OpenAI-compatible relay. Overrides are applied to actual requests, not engine hints.
use crate::config::{Profile, Provider};
use anyhow::{Context, Result, ensure};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    task::{JoinHandle, JoinSet},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub version: u32,
    pub output_tokens: u32,
    /// Reserved inside total output; thinking may not consume this action headroom.
    pub action_headroom: u32,
    pub reasoning_tokens: Option<u32>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<i32>,
    pub min_p: Option<f64>,
    /// Explicit external server capability; automatically true for owned llama.cpp.
    #[serde(default)]
    pub llama_extensions: bool,
    #[serde(default)]
    pub total_generated_tokens: Option<u32>,
    #[serde(default)]
    pub max_requests: Option<u32>,
    #[serde(default)]
    pub output_parameter: OutputParameter,
    /// One reasoning choice, translated per server dialect (A01).
    #[serde(default)]
    pub thinking: Thinking,
    /// Which request fields this server understands for reasoning control.
    #[serde(default)]
    pub dialect: Dialect,
    /// A02 open parameters. Each is an explicit choice; unset sends nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_penalty: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typical_p: Option<f64>,
    /// A JSON object merged into every request last. Keys Alt owns are refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_body: Option<String>,
    /// Ollama only: `server-default` stops Alt sending `options.num_ctx`.
    #[serde(default)]
    pub ollama_context: OllamaContext,
    /// Ollama only: how long the server keeps the model loaded (`5m`, `-1`, `0`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum OllamaContext {
    /// Send the profile context as `options.num_ctx`.
    #[default]
    Profile,
    /// Send nothing; the server or model tag decides.
    ServerDefault,
}

/// Request fields Alt owns. `extra_body` may not replace them.
pub const RESERVED_BODY_KEYS: [&str; 9] = [
    "model",
    "messages",
    "tools",
    "tool_choice",
    "stream",
    "stream_options",
    "max_tokens",
    "max_completion_tokens",
    "n",
];

/// `default | off | low | medium | high | budget:N`. Default sends nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Thinking {
    #[default]
    Default,
    Off,
    Low,
    Medium,
    High,
    Budget(u32),
}
impl Thinking {
    pub const CHOICES: [Self; 5] = [
        Self::Default,
        Self::Off,
        Self::Low,
        Self::Medium,
        Self::High,
    ];
    pub fn label(self) -> String {
        match self {
            Self::Default => "Server default".into(),
            Self::Off => "Off".into(),
            Self::Low => "Low".into(),
            Self::Medium => "Medium".into(),
            Self::High => "High".into(),
            Self::Budget(n) => format!("Budget of {n} tokens"),
        }
    }
    fn effort(self) -> Option<&'static str> {
        match self {
            Self::Low => Some("low"),
            Self::Medium => Some("medium"),
            Self::High => Some("high"),
            _ => None,
        }
    }
}
impl std::str::FromStr for Thinking {
    type Err = anyhow::Error;
    fn from_str(text: &str) -> Result<Self> {
        let text = text.trim().to_ascii_lowercase();
        Ok(match text.as_str() {
            "" | "default" => Self::Default,
            "off" | "none" | "false" => Self::Off,
            "low" => Self::Low,
            "medium" => Self::Medium,
            "high" | "on" | "true" => Self::High,
            other => Self::Budget(
                other
                    .strip_prefix("budget:")
                    .or_else(|| other.strip_prefix("budget("))
                    .map(|n| n.trim_end_matches(')'))
                    .unwrap_or(other)
                    .parse()
                    .map_err(|_| {
                        anyhow::anyhow!(
                            "Thinking must be default, off, low, medium, high or budget:N"
                        )
                    })?,
            ),
        })
    }
}
impl TryFrom<String> for Thinking {
    type Error = anyhow::Error;
    fn try_from(text: String) -> Result<Self> {
        text.parse()
    }
}
impl From<Thinking> for String {
    fn from(value: Thinking) -> Self {
        match value {
            Thinking::Budget(n) => format!("budget:{n}"),
            other => other.label().to_ascii_lowercase().replace("server ", ""),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Dialect {
    /// Owned/declared llama.cpp, Ollama by provider, otherwise OpenAI-style.
    #[default]
    Auto,
    LlamaCpp,
    Ollama,
    /// vLLM, SGLang and NVIDIA NIM: `chat_template_kwargs`.
    ChatTemplate,
    /// OpenAI-style `reasoning_effort` only.
    OpenaiEffort,
}
impl Dialect {
    pub const ALL: [Self; 5] = [
        Self::Auto,
        Self::LlamaCpp,
        Self::Ollama,
        Self::ChatTemplate,
        Self::OpenaiEffort,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Automatic",
            Self::LlamaCpp => "llama.cpp",
            Self::Ollama => "Ollama",
            Self::ChatTemplate => "vLLM / SGLang / NVIDIA NIM",
            Self::OpenaiEffort => "OpenAI-style reasoning_effort",
        }
    }
    pub fn fields(self) -> &'static str {
        match self {
            Self::Auto => "chosen from the connection",
            Self::LlamaCpp => "chat_template_kwargs.enable_thinking + reasoning_budget_tokens",
            Self::Ollama => "think + reasoning_effort",
            Self::ChatTemplate => {
                "chat_template_kwargs.enable_thinking/thinking + reasoning_effort"
            }
            Self::OpenaiEffort => "reasoning_effort",
        }
    }
    pub fn saved_name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::LlamaCpp => "llama-cpp",
            Self::Ollama => "ollama",
            Self::ChatTemplate => "chat-template",
            Self::OpenaiEffort => "openai-effort",
        }
    }
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum OutputParameter {
    #[default]
    MaxTokens,
    MaxCompletionTokens,
}
impl Settings {
    pub fn legacy(context: u32) -> Self {
        Self {
            version: 1,
            output_tokens: (context / 4).min(2048),
            action_headroom: 256,
            reasoning_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            min_p: None,
            llama_extensions: false,
            total_generated_tokens: None,
            max_requests: None,
            output_parameter: OutputParameter::MaxTokens,
            thinking: Thinking::Default,
            dialect: Dialect::Auto,
            repeat_penalty: None,
            presence_penalty: None,
            frequency_penalty: None,
            seed: None,
            stop: Vec::new(),
            typical_p: None,
            extra_body: None,
            ollama_context: OllamaContext::Profile,
            keep_alive: None,
        }
    }
    /// Parse and check `extra_body`: a JSON object without Alt-owned keys.
    pub fn extra_object(&self) -> Result<serde_json::Map<String, Value>> {
        let Some(text) = self.extra_body.as_deref().filter(|t| !t.trim().is_empty()) else {
            return Ok(serde_json::Map::new());
        };
        ensure!(
            text.len() <= 16 * 1024,
            "Extra request JSON must be at most 16 KiB"
        );
        let value: Value =
            serde_json::from_str(text).context("Extra request fields must be valid JSON")?;
        let object = value
            .as_object()
            .cloned()
            .context("Extra request fields must be a JSON object, for example {\"top_a\":0.1}")?;
        for key in object.keys() {
            ensure!(
                !RESERVED_BODY_KEYS.contains(&key.as_str()),
                "Alt owns `{key}` (model, messages, tools, streaming and output-token fields); remove it from the extra request fields"
            );
        }
        Ok(object)
    }
    /// Sampling and penalty fields in this server's spelling, or an error naming
    /// a choice the server cannot accept.
    fn parameter_fields(&self, p: &Profile) -> Result<serde_json::Map<String, Value>> {
        let dialect = self.resolved_dialect(p);
        let mut fields = serde_json::Map::new();
        if let Some(v) = self.presence_penalty {
            fields.insert("presence_penalty".into(), json!(v));
        }
        if let Some(v) = self.frequency_penalty {
            fields.insert("frequency_penalty".into(), json!(v));
        }
        if let Some(v) = self.seed {
            fields.insert("seed".into(), json!(v));
        }
        if !self.stop.is_empty() {
            fields.insert("stop".into(), json!(self.stop));
        }
        if let Some(v) = self.repeat_penalty {
            let name = match dialect {
                Dialect::LlamaCpp | Dialect::Ollama => "repeat_penalty",
                Dialect::ChatTemplate => "repetition_penalty",
                _ => anyhow::bail!(
                    "This server's OpenAI-style API has no repeat penalty; use presence/frequency penalty, pick its dialect, or put the field in extra request JSON"
                ),
            };
            fields.insert(name.into(), json!(v));
        }
        if let Some(v) = self.typical_p {
            ensure!(
                matches!(dialect, Dialect::LlamaCpp | Dialect::Ollama),
                "typical_p is a llama.cpp/Ollama sampler; pick that dialect or use extra request JSON"
            );
            fields.insert("typical_p".into(), json!(v));
        }
        if p.provider == Provider::Ollama {
            if self.ollama_context == OllamaContext::Profile {
                fields.insert("options".into(), json!({"num_ctx": p.context_tokens}));
            }
            if let Some(v) = &self.keep_alive {
                fields.insert("keep_alive".into(), json!(v));
            }
        }
        Ok(fields)
    }
    pub fn resolved_dialect(&self, p: &Profile) -> Dialect {
        match self.dialect {
            Dialect::Auto if p.local_model.is_some() || self.llama_extensions => Dialect::LlamaCpp,
            Dialect::Auto if p.provider == Provider::Ollama => Dialect::Ollama,
            Dialect::Auto => Dialect::OpenaiEffort,
            chosen => chosen,
        }
    }
    /// The legacy thinking budget remains a budget choice on llama.cpp.
    pub fn effective_thinking(&self) -> Thinking {
        match (self.thinking, self.reasoning_tokens) {
            (Thinking::Default, Some(n)) => Thinking::Budget(n),
            (choice, _) => choice,
        }
    }
    /// Penalties, seed, stop, Ollama context, then `extra_body` merged last.
    pub fn apply_parameters(
        &self,
        p: &Profile,
        obj: &mut serde_json::Map<String, Value>,
    ) -> Result<()> {
        for key in [
            "presence_penalty",
            "frequency_penalty",
            "seed",
            "stop",
            "repeat_penalty",
            "repetition_penalty",
            "typical_p",
            "keep_alive",
            "options",
        ] {
            obj.remove(key);
        }
        for (key, value) in self.parameter_fields(p)? {
            obj.insert(key, value);
        }
        for (key, value) in self.extra_object()? {
            obj.insert(key, value);
        }
        Ok(())
    }
    /// Write the reasoning fields for this dialect into a request and return a
    /// record of exactly what was sent, for the receipt.
    pub fn apply_thinking(
        &self,
        p: &Profile,
        obj: &mut serde_json::Map<String, Value>,
        allowance: u32,
    ) -> Value {
        for key in [
            "reasoning_budget_tokens",
            "reasoning_effort",
            "think",
            "chat_template_kwargs",
        ] {
            obj.remove(key);
        }
        let dialect = self.resolved_dialect(p);
        let thinking = self.effective_thinking();
        let room = allowance.saturating_sub(self.action_headroom);
        let mut sent = serde_json::Map::new();
        match (dialect, thinking) {
            (_, Thinking::Default) => {}
            // Older saved budgets keep their exact request shape.
            (Dialect::LlamaCpp, Thinking::Budget(n)) if self.thinking == Thinking::Default => {
                sent.insert("reasoning_budget_tokens".into(), json!(n.min(room)));
            }
            (Dialect::LlamaCpp, choice) => {
                let budget = match choice {
                    Thinking::Off => 0,
                    Thinking::Low => room / 4,
                    Thinking::Medium => room / 2,
                    Thinking::Budget(n) => n.min(room),
                    _ => room,
                };
                sent.insert(
                    "chat_template_kwargs".into(),
                    json!({"enable_thinking": choice != Thinking::Off}),
                );
                sent.insert("reasoning_budget_tokens".into(), json!(budget));
            }
            (Dialect::Ollama, choice) => {
                sent.insert("think".into(), json!(choice != Thinking::Off));
                sent.insert(
                    "reasoning_effort".into(),
                    json!(choice.effort().unwrap_or("none")),
                );
            }
            (Dialect::ChatTemplate, choice) => {
                let on = choice != Thinking::Off;
                sent.insert(
                    "chat_template_kwargs".into(),
                    json!({"enable_thinking": on, "thinking": on}),
                );
                if let Some(effort) = choice.effort() {
                    sent.insert("reasoning_effort".into(), json!(effort));
                }
            }
            (_, choice) => {
                sent.insert(
                    "reasoning_effort".into(),
                    json!(choice.effort().unwrap_or("none")),
                );
            }
        }
        for (key, value) in &sent {
            obj.insert(key.clone(), value.clone());
        }
        json!({"dialect":dialect.saved_name(),"thinking":String::from(thinking),"fields":sent})
    }
    pub fn validate(&self, p: &Profile) -> Result<()> {
        ensure!(self.version == 1, "Unsupported inference settings version");
        ensure!(
            self.output_tokens >= 128 && self.output_tokens < p.context_tokens,
            "Output allowance must be at least 128 and smaller than the context window"
        );
        ensure!(
            self.action_headroom >= 64 && self.action_headroom <= self.output_tokens,
            "Action headroom must be 64..total output tokens"
        );
        ensure!(
            self.reasoning_tokens
                .is_none_or(|n| n <= self.output_tokens - self.action_headroom),
            "Thinking allocation must leave the selected action headroom inside total output"
        );
        ensure!(
            self.temperature
                .is_none_or(|n| n.is_finite() && (0.0..=5.0).contains(&n)),
            "Temperature must be 0..5"
        );
        ensure!(
            self.top_p
                .is_none_or(|n| n.is_finite() && n > 0.0 && n <= 1.0),
            "top_p must be >0 and <=1"
        );
        ensure!(
            self.min_p
                .is_none_or(|n| n.is_finite() && (0.0..=1.0).contains(&n)),
            "min_p must be 0..1"
        );
        ensure!(
            self.top_k.is_none_or(|n| n >= -1),
            "top_k must be -1 or greater"
        );
        for (name, value) in [
            ("presence_penalty", self.presence_penalty),
            ("frequency_penalty", self.frequency_penalty),
        ] {
            ensure!(
                value.is_none_or(|n| n.is_finite() && (-2.0..=2.0).contains(&n)),
                "{name} must be between -2 and 2"
            );
        }
        ensure!(
            self.repeat_penalty
                .is_none_or(|n| n.is_finite() && (0.0..=5.0).contains(&n)),
            "repeat penalty must be 0..5 (1 means off)"
        );
        ensure!(
            self.typical_p
                .is_none_or(|n| n.is_finite() && n > 0.0 && n <= 1.0),
            "typical_p must be >0 and <=1"
        );
        ensure!(
            self.stop.len() <= 16 && self.stop.iter().all(|s| !s.is_empty() && s.len() <= 256),
            "Use up to 16 non-empty stop sequences of at most 256 bytes"
        );
        ensure!(
            self.keep_alive.as_deref().is_none_or(|k| {
                let k = k.trim();
                !k.is_empty()
                    && k.len() <= 16
                    && k.chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
            }),
            "keep_alive looks like 5m, 1h, 0 or -1"
        );
        self.extra_object()?;
        self.parameter_fields(p)?;
        ensure!(
            self.total_generated_tokens.is_none_or(|n| n > 0)
                && self.max_requests.is_none_or(|n| n > 0),
            "Shared generation/request allowances must be positive"
        );
        let dialect = self.resolved_dialect(p);
        ensure!(
            !matches!(self.effective_thinking(), Thinking::Budget(_))
                || dialect == Dialect::LlamaCpp,
            "A thinking token budget needs a llama.cpp server; {} accepts off, low, medium or high",
            dialect.label()
        );
        ensure!(
            !matches!(self.thinking, Thinking::Budget(n) if n > self.output_tokens - self.action_headroom),
            "Thinking budget must leave the selected action headroom inside total output"
        );
        ensure!(
            p.local_model.is_some()
                || self.llama_extensions
                || dialect == Dialect::LlamaCpp
                || (self.top_k.is_none()
                    && self.min_p.is_none()
                    && self.reasoning_tokens.is_none()),
            "This external endpoint has no declared llama.cpp extensions; clear top_k/min_p/reasoning budget or explicitly qualify that server capability"
        );
        Ok(())
    }
    pub fn apply(&self, p: &Profile, body: &mut Value) -> Result<()> {
        self.validate(p)?;
        ensure!(
            body["model"].as_str() == Some(&p.model),
            "Engine requested a different model; no fallback is allowed"
        );
        let obj = body
            .as_object_mut()
            .context("Chat request must be an object")?;
        obj.remove("max_completion_tokens");
        obj.remove("options");
        obj.remove("max_tokens");
        obj.insert(
            match self.output_parameter {
                OutputParameter::MaxTokens => "max_tokens",
                OutputParameter::MaxCompletionTokens => "max_completion_tokens",
            }
            .into(),
            json!(self.output_tokens),
        );
        // Server defaults are intentional when no override is selected. Engine defaults must not win.
        for (name, value) in [
            ("temperature", self.temperature.map(|n| json!(n))),
            ("top_p", self.top_p.map(|n| json!(n))),
            ("top_k", self.top_k.map(|n| json!(n))),
            ("min_p", self.min_p.map(|n| json!(n))),
        ] {
            obj.remove(name);
            if let Some(value) = value {
                obj.insert(name.into(), value);
            }
        }
        self.apply_thinking(p, obj, self.output_tokens);
        self.apply_parameters(p, obj)?;
        if obj.get("stream") == Some(&json!(true)) {
            obj.insert("stream_options".into(), json!({"include_usage":true}));
        }
        Ok(())
    }
    pub fn accounting(&self, p: &Profile) -> Value {
        json!({"settings":self,"input_allowance":p.context_tokens-self.output_tokens,"total_output_allowance":self.output_tokens,"reasoning_allowance":self.reasoning_tokens,"action_headroom":self.action_headroom,"accounting":"Owned runtime measures the full rendered chat/tools before forwarding when its template/tokenizer endpoints are available. Each receipt records measurement or unknown; server usage is authoritative. External native context is unqualified.","native_context":"Owned runtime configured; external server must be inspected separately","thinking_support":"Budget only affects a supported thinking parser; non-thinking checkpoints do not gain a thinking mode.","thinking_control":{"choice":String::from(self.effective_thinking()),"dialect":self.resolved_dialect(p).label(),"fields":self.resolved_dialect(p).fields()},"request_parameters":self.parameter_fields(p).unwrap_or_default(),"extra_body":self.extra_object().unwrap_or_default()})
    }
}

pub struct Relay {
    pub endpoint: String,
    profile: Profile,
    listener: JoinHandle<()>,
    pub evidence: PathBuf,
    live: Live,
}
#[derive(Clone)]
struct Live {
    budget: std::sync::Arc<tokio::sync::Mutex<(u32, u32)>>,
    status: tokio::sync::watch::Sender<Status>,
    /// Explicit per-connection thinking choice made after a cutoff.
    thinking: std::sync::Arc<std::sync::Mutex<Option<Thinking>>>,
}
impl Live {
    fn settings(&self, p: &Profile) -> Settings {
        let mut settings = p.effective_inference();
        if let Some(choice) = *self.thinking.lock().expect("thinking override") {
            settings.thinking = choice;
            if choice != Thinking::Default {
                settings.reasoning_tokens = None;
            }
        }
        settings
    }
}

/// Observed relay activity. Waiting includes server queueing and prompt processing;
/// it must not be presented as measured reasoning or GPU utilization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Ready,
    MeasuringInput,
    WaitingForOutput,
    ReceivingOutput,
    AllowanceExhausted,
    InputTooLarge,
    Interrupted,
    /// The response hit the output limit with reasoning only: no answer, no tool call.
    ReasoningCutoff,
}
impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "Ready for a model request",
            Self::MeasuringInput => "Measuring full prompt and tools",
            Self::WaitingForOutput => "Waiting for model output (queue or prompt processing)",
            Self::ReceivingOutput => "Receiving model output",
            Self::AllowanceExhausted => "Model allowance exhausted",
            Self::InputTooLarge => "Prompt exceeds the input reserve",
            Self::Interrupted => "Model request interrupted",
            Self::ReasoningCutoff => "Output allowance spent on thinking; no answer or tool call",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    pub connection_id: String,
    pub stage: Stage,
    pub requests_issued: u64,
    pub remaining_requests: Option<u32>,
    pub remaining_generated_tokens: Option<u32>,
    pub input_tokens: Option<u64>,
    pub input_allowance: u32,
    pub output_allowance: u32,
    pub action_headroom: u32,
    pub reasoning_allowance: Option<u32>,
    /// Responses on this connection that ended in a reasoning-only cutoff.
    #[serde(default)]
    pub reasoning_cutoffs: u32,
    /// Ollama only: the context the loaded model actually uses, read from
    /// `/api/ps` after a response. `None` until measured or when not reported.
    #[serde(default)]
    pub ollama_context: Option<u64>,
}
impl Status {
    fn new(p: &Profile) -> Self {
        let s = p.effective_inference();
        Self {
            connection_id: uuid::Uuid::new_v4().to_string(),
            stage: Stage::Ready,
            requests_issued: 0,
            remaining_requests: s.max_requests,
            remaining_generated_tokens: s.total_generated_tokens,
            input_tokens: None,
            input_allowance: p.context_tokens - s.output_tokens,
            output_allowance: s.output_tokens,
            action_headroom: s.action_headroom,
            reasoning_allowance: s.reasoning_tokens,
            reasoning_cutoffs: 0,
            ollama_context: None,
        }
    }
}
impl Relay {
    pub async fn start(root: &Path, p: &Profile) -> Result<Self> {
        p.validate()?;
        let socket = TcpListener::bind("127.0.0.1:0").await?;
        let secret = uuid::Uuid::new_v4().to_string();
        let endpoint = format!("http://{}/{secret}/v1", socket.local_addr()?);
        let profile = p.clone();
        let key = p.key()?;
        let evidence = root
            .join("inference")
            .join(uuid::Uuid::new_v4().to_string());
        crate::config::private_dir(&evidence)?;
        crate::config::atomic_write(
            &evidence.join("effective.json"),
            &serde_json::to_vec_pretty(&p.effective_inference().accounting(p))?,
        )?;
        let directory = evidence.clone();
        let settings = p.effective_inference();
        let budget = std::sync::Arc::new(tokio::sync::Mutex::new((
            settings.total_generated_tokens.unwrap_or(u32::MAX),
            settings.max_requests.unwrap_or(u32::MAX),
        )));
        let (status, _) = tokio::sync::watch::channel(Status::new(p));
        let live = Live {
            budget,
            status,
            thinking: Default::default(),
        };
        let service = live.clone();
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(15))
            .build()?;
        let listener = tokio::spawn(async move {
            let mut children = JoinSet::new();
            loop {
                tokio::select! {
                    accepted=socket.accept()=>match accepted {
                        Ok((stream,_))=> {let p=profile.clone();let key=key.clone();let secret=secret.clone();let evidence=directory.clone();let client=client.clone();let live=service.clone();children.spawn(async move {
                            if let Err(e)=serve(stream,&p,key.as_deref(),&secret,&evidence,&client,&live).await {
                                live.status.send_modify(|s| s.stage=Stage::Interrupted);
                                let _=crate::config::atomic_write(&evidence.join(format!("error-{}.json",uuid::Uuid::new_v4())),&serde_json::to_vec(&json!({"error":e.to_string()})).unwrap_or_default());
                            }
                        });}, Err(_)=>break
                    },
                    _=children.join_next(),if !children.is_empty()=>{}
                }
            }
        });
        Ok(Self {
            endpoint,
            profile: p.clone(),
            listener,
            evidence,
            live,
        })
    }
    pub fn status(&self) -> Status {
        self.live.status.borrow().clone()
    }
    /// Change the thinking choice for the rest of this connection, after the
    /// user picked it (for example "thinking off" after a cutoff). Recorded as
    /// evidence; spent tokens stay charged and no request is sent.
    pub fn set_thinking(&self, choice: Thinking) -> Result<Status> {
        let profile = &self.profile;
        let mut next = profile.effective_inference();
        next.thinking = choice;
        if choice != Thinking::Default {
            next.reasoning_tokens = None;
        }
        next.validate(profile)?;
        crate::config::atomic_write(
            &self
                .evidence
                .join(format!("thinking-{}.json", uuid::Uuid::new_v4())),
            &serde_json::to_vec_pretty(
                &json!({"schema":1,"thinking":String::from(choice),"dialect":next.resolved_dialect(profile).saved_name(),"scope":"Explicit user choice for this connection; model, provider and spent costs unchanged; no request issued"}),
            )?,
        )?;
        *self.live.thinking.lock().expect("thinking override") = Some(choice);
        self.live.status.send_modify(|s| {
            if s.stage == Stage::ReasoningCutoff {
                s.stage = Stage::Ready;
            }
        });
        Ok(self.status())
    }
    /// Adds only a finite, explicitly requested allowance. Existing costs remain
    /// charged; configuration and model identity are unchanged.
    pub fn add_allowance(&self, requests: u32, tokens: u32) -> Result<Status> {
        ensure!(
            requests > 0 && tokens > 0,
            "Add a positive request and token allowance"
        );
        let mut budget = self.live.budget.try_lock().context(
            "A model request is still finishing. Wait or reconnect before adding allowance",
        )?;
        let current = self.status();
        ensure!(
            current.remaining_requests.is_some() && current.remaining_generated_tokens.is_some(),
            "This connection has no finite shared allowance to extend. Configure both limits in Model effort for the next connection"
        );
        let next_tokens = budget
            .0
            .checked_add(tokens)
            .context("Token allowance is too large")?;
        let next_requests = budget
            .1
            .checked_add(requests)
            .context("Request allowance is too large")?;
        let grant = json!({"schema":1,"requests_added":requests,"generated_tokens_added":tokens,"requests_issued":current.requests_issued,"remaining_requests_before":budget.1,"remaining_generated_tokens_before":budget.0,"model_unchanged":true,"scope":"Explicit user addition to this connection; no spent cost refunded and no model request issued"});
        crate::config::atomic_write(
            &self
                .evidence
                .join(format!("grant-{}.json", uuid::Uuid::new_v4())),
            &serde_json::to_vec_pretty(&grant)?,
        )?;
        *budget = (next_tokens, next_requests);
        self.live.status.send_modify(|s| {
            s.remaining_requests = Some(next_requests);
            s.remaining_generated_tokens = Some(next_tokens);
            if s.stage == Stage::AllowanceExhausted {
                s.stage = Stage::Ready;
            }
        });
        Ok(self.status())
    }
}
impl Drop for Relay {
    fn drop(&mut self) {
        self.listener.abort();
    }
}
async fn bounded_json(response: reqwest::Response) -> Result<Value> {
    let mut stream = response.error_for_status()?.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        ensure!(
            bytes.len() + chunk.len() <= 8 * 1024 * 1024,
            "Tokenizer response exceeds 8 MiB"
        );
        bytes.extend_from_slice(&chunk);
    }
    Ok(serde_json::from_slice(&bytes)?)
}
/// Uses the same pinned server chat parser as inference, including tool schemas.
async fn input_accounting(p: &Profile, body: &Value) -> Value {
    if p.local_model.is_none() {
        return json!({"tokens":null,"scope":"External endpoint; no qualified full-chat tokenizer"});
    }
    let measured = tokio::time::timeout(Duration::from_secs(5), async {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        let base = p.endpoint.trim_end_matches('/').trim_end_matches("/v1");
        let key = p.key()?;
        let mut request = client.post(format!("{base}/apply-template")).json(body);
        if let Some(key) = &key {
            request = request.bearer_auth(key);
        }
        let rendered = bounded_json(request.send().await?).await?;
        let prompt = rendered
            .get("prompt")
            .context("Runtime omitted rendered prompt")?;
        let mut request = client
            .post(format!("{base}/tokenize"))
            .json(&json!({"content":prompt,"add_special":false,"parse_special":true}));
        if let Some(key) = &key {
            request = request.bearer_auth(key);
        }
        let tokens = bounded_json(request.send().await?).await?;
        Ok::<usize, anyhow::Error>(
            tokens["tokens"]
                .as_array()
                .context("Runtime omitted tokens")?
                .len(),
        )
    })
    .await;
    match measured {
        Ok(Ok(tokens)) => {
            json!({"tokens":tokens,"scope":"Selected owned runtime full rendered chat, tools, special tokens and generation prefix"})
        }
        _ => {
            json!({"tokens":null,"scope":"Owned runtime full-chat measurement unavailable; server usage remains authoritative"})
        }
    }
}
pub fn response_complete(raw: &[u8], streaming: bool) -> bool {
    if !streaming {
        return serde_json::from_slice::<Value>(raw).ok().is_some_and(|v| {
            v.get("error").is_none()
                && v["choices"][0]["message"].is_object()
                && v["choices"][0]["finish_reason"].as_str().is_some()
        });
    }
    let Ok(text) = std::str::from_utf8(raw) else {
        return false;
    };
    let mut done = false;
    let mut finish = false;
    for data in text.lines().filter_map(|s| s.strip_prefix("data:")) {
        if done {
            return false;
        }
        if data.trim() == "[DONE]" {
            done = true;
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(data.trim()) else {
            return false;
        };
        if v.get("error").is_some() {
            return false;
        }
        finish |= v["choices"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|r| r["finish_reason"].as_str().is_some()));
    }
    done && finish
}
/// A response that stopped at the output limit after producing reasoning but
/// neither content nor a tool call. Spent tokens stay charged; recovery is the
/// user's choice (thinking off, or a larger allowance), never a silent retry.
pub fn reasoning_cutoff(raw: &[u8], streaming: bool) -> bool {
    let mut reasoning = false;
    let mut answer = false;
    let mut length = false;
    let mut observe = |part: &Value, finish: &Value| {
        for key in ["reasoning_content", "reasoning", "thinking"] {
            reasoning |= part[key].as_str().is_some_and(|t| !t.trim().is_empty());
        }
        answer |= part["content"]
            .as_str()
            .is_some_and(|t| !t.trim().is_empty())
            || part["tool_calls"].as_array().is_some_and(|c| !c.is_empty());
        length |= finish.as_str() == Some("length");
    };
    if streaming {
        let text = String::from_utf8_lossy(raw);
        for data in text.lines().filter_map(|l| l.strip_prefix("data:")) {
            if let Ok(v) = serde_json::from_str::<Value>(data.trim()) {
                for choice in v["choices"].as_array().into_iter().flatten() {
                    observe(&choice["delta"], &choice["finish_reason"]);
                }
            }
        }
    } else if let Ok(v) = serde_json::from_slice::<Value>(raw) {
        for choice in v["choices"].as_array().into_iter().flatten() {
            observe(&choice["message"], &choice["finish_reason"]);
        }
    }
    length && reasoning && !answer
}
/// Read the loaded context of `model` from Ollama's `/api/ps`.
pub async fn ollama_loaded_context(
    client: &reqwest::Client,
    p: &Profile,
    key: Option<&str>,
) -> Option<u64> {
    let mut request = client
        .get(format!("{}/api/ps", p.endpoint.trim_end_matches('/')))
        .timeout(Duration::from_secs(3));
    if let Some(key) = key {
        request = request.bearer_auth(key);
    }
    let value = bounded_json(request.send().await.ok()?).await.ok()?;
    value["models"].as_array()?.iter().find_map(|m| {
        (m["name"].as_str() == Some(&p.model) || m["model"].as_str() == Some(&p.model))
            .then(|| m["context_length"].as_u64())
            .flatten()
    })
}
pub fn cost(evidence: &Path) -> Result<Value> {
    let mut requests = 0u32;
    let mut charged = 0u64;
    let mut generated = Some(0u64);
    for entry in std::fs::read_dir(evidence)? {
        let path = entry?.path();
        if !path.to_string_lossy().ends_with("-receipt.json") {
            continue;
        }
        let receipt: Value = serde_json::from_slice(&std::fs::read(path)?)?;
        requests += 1;
        charged = charged.saturating_add(
            receipt["charged_generated_tokens"]
                .as_u64()
                .context("Issued receipt lacks reserved cost")?,
        );
        generated = generated
            .zip(receipt["generated_tokens"].as_u64())
            .map(|(a, b)| a.saturating_add(b));
    }
    Ok(
        json!({"requests":requests,"charged_generated_tokens":charged,"known_generated_tokens":generated,"scope":"Includes conservatively reserved costs of interrupted or unmeasured requests"}),
    )
}
async fn serve(
    stream: TcpStream,
    p: &Profile,
    key: Option<&str>,
    secret: &str,
    evidence: &Path,
    client: &reqwest::Client,
    live: &Live,
) -> Result<()> {
    let (budget, live_status) = (&live.budget, &live.status);
    let settings = live.settings(p);
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let request = tokio::time::timeout(Duration::from_secs(20), async {
        let mut head = Vec::new();
        loop {
            let mut line = Vec::new();
            (&mut reader)
                .take(16385)
                .read_until(b'\n', &mut line)
                .await?;
            ensure!(
                !line.is_empty() && head.len() + line.len() <= 16384,
                "Invalid/oversized request headers"
            );
            let done = line == b"\r\n";
            head.extend(line);
            if done {
                break;
            }
        }
        let text = std::str::from_utf8(&head)?;
        let first = text.lines().next().unwrap_or("");
        ensure!(
            first == format!("POST /{secret}/v1/chat/completions HTTP/1.1"),
            "Unknown inference route"
        );
        let mut length = None;
        for line in text.lines().skip(1) {
            if let Some((name, value)) = line.split_once(':') {
                ensure!(
                    !name.eq_ignore_ascii_case("transfer-encoding"),
                    "Chunked request bodies are unsupported"
                );
                if name.eq_ignore_ascii_case("content-length") {
                    ensure!(length.is_none(), "Duplicate Content-Length");
                    length = Some(value.trim().parse::<usize>()?);
                }
            }
        }
        let length = length.context("Missing Content-Length")?;
        ensure!(length <= 8 * 1024 * 1024, "Inference request exceeds 8 MiB");
        let mut bytes = vec![0; length];
        reader.read_exact(&mut bytes).await?;
        let mut body: Value = serde_json::from_slice(&bytes)?;
        settings.apply(p, &mut body)?;
        Ok::<Value, anyhow::Error>(body)
    })
    .await;
    let mut body = match request {
        Ok(Ok(body)) => body,
        other => {
            let msg = format!("Invalid inference request: {other:?}");
            let bytes = serde_json::to_vec(&json!({"error":{"message":msg}}))?;
            writer.write_all(format!("HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len()).as_bytes()).await?;
            writer.write_all(&bytes).await?;
            return Ok(());
        }
    };
    let mut budget = budget.lock().await;
    if budget.0 == 0 || budget.1 == 0 {
        live_status.send_modify(|s| s.stage = Stage::AllowanceExhausted);
        let bytes = serde_json::to_vec(
            &json!({"error":{"message":"Selected shared effort allowance exhausted; no extra model call was sent"}}),
        )?;
        writer.write_all(format!("HTTP/1.1 429 Too Many Requests\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len()).as_bytes()).await?;
        writer.write_all(&bytes).await?;
        return Ok(());
    }
    let allowance = settings.output_tokens.min(budget.0);
    let token_field = match settings.output_parameter {
        OutputParameter::MaxTokens => "max_tokens",
        OutputParameter::MaxCompletionTokens => "max_completion_tokens",
    };
    body[token_field] = json!(allowance);
    let thinking_sent = settings.apply_thinking(
        p,
        body.as_object_mut()
            .context("Chat request must be an object")?,
        allowance,
    );
    // Extra request fields stay last, after the per-call thinking allocation.
    for (key, value) in settings.extra_object()? {
        body[key] = value;
    }
    // Measure the actual allocated body, including any reduced thinking budget.
    // Input rejection leaves the shared reservation untouched.
    live_status.send_modify(|s| s.stage = Stage::MeasuringInput);
    let input = input_accounting(p, &body).await;
    live_status.send_modify(|s| {
        s.input_tokens = input["tokens"].as_u64();
        s.output_allowance = allowance;
        s.reasoning_allowance = body["reasoning_budget_tokens"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok());
    });
    if input["tokens"]
        .as_u64()
        .is_some_and(|n| n > (p.context_tokens - settings.output_tokens) as u64)
    {
        live_status.send_modify(|s| s.stage = Stage::InputTooLarge);
        crate::config::atomic_write(
            &evidence.join(format!("rejected-{}.json", uuid::Uuid::new_v4())),
            &serde_json::to_vec(
                &json!({"reason":"input reserve exceeded","input_accounting":input,"request":body,"issued":false}),
            )?,
        )?;
        let bytes = serde_json::to_vec(
            &json!({"error":{"message":"Full rendered chat and tools exceed the selected input reserve. Shorten the input, reduce output allocation, or increase measured native context."}}),
        )?;
        writer.write_all(format!("HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len()).as_bytes()).await?;
        writer.write_all(&bytes).await?;
        return Ok(());
    }
    budget.0 -= allowance;
    budget.1 -= 1;
    live_status.send_modify(|s| {
        s.requests_issued += 1;
        if s.remaining_requests.is_some() {
            s.remaining_requests = Some(budget.1);
        }
        if s.remaining_generated_tokens.is_some() {
            s.remaining_generated_tokens = Some(budget.0);
        }
        s.stage = Stage::WaitingForOutput;
    });
    let id = uuid::Uuid::new_v4().to_string();
    let receipt = evidence.join(format!("{id}-receipt.json"));
    crate::config::atomic_write(
        &receipt,
        &serde_json::to_vec(
            &json!({"complete":false,"status":"issued_or_interrupted","charged_generated_tokens":allowance,"generated_tokens":null,"allowance":allowance,"input_accounting":input,"request_sha256":crate::project::digest(&serde_json::to_vec(&body)?)}),
        )?,
    )?;
    crate::config::atomic_write(
        &evidence.join(format!("{id}-request.json")),
        &serde_json::to_vec(&body)?,
    )?;
    let url = match p.provider {
        Provider::Openai => format!("{}/chat/completions", p.endpoint.trim_end_matches('/')),
        Provider::Ollama => format!("{}/v1/chat/completions", p.endpoint.trim_end_matches('/')),
    };
    let mut req = client.post(url).json(&body);
    if let Some(key) = key {
        req = req.bearer_auth(key);
    }
    let response = match req.send().await {
        Ok(v) => v,
        Err(_) => {
            writer
                .write_all(
                    b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await?;
            anyhow::bail!("Selected provider connection failed; inspect the selected endpoint");
        }
    };
    let status = response.status();
    let content = if body["stream"] == true {
        "text/event-stream"
    } else {
        "application/json"
    };
    writer.write_all(format!("HTTP/1.1 {} Provider\r\nContent-Type: {content}\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",status.as_u16()).as_bytes()).await?;
    let mut stream = response.bytes_stream();
    let mut raw_file = tokio::fs::File::create(evidence.join(format!("{id}-response.raw"))).await?;
    let mut captured = Vec::new();
    let mut truncated = false;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let observed_stage = live_status.borrow().stage;
        if !chunk.is_empty() && observed_stage != Stage::ReceivingOutput {
            live_status.send_modify(|s| s.stage = Stage::ReceivingOutput);
        }
        if !truncated && captured.len() + chunk.len() <= 8 * 1024 * 1024 {
            captured.extend_from_slice(&chunk);
            raw_file.write_all(&chunk).await?;
        } else {
            truncated = true;
        }
        writer
            .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
            .await?;
        writer.write_all(&chunk).await?;
        writer.write_all(b"\r\n").await?;
        writer.flush().await?;
    }
    raw_file.sync_all().await?;
    let usage = if body["stream"] == true {
        String::from_utf8_lossy(&captured)
            .lines()
            .filter_map(|l| l.strip_prefix("data:"))
            .filter_map(|s| serde_json::from_str::<Value>(s.trim()).ok())
            .filter_map(|v| v.get("usage").filter(|u| u.is_object()).cloned())
            .next_back()
    } else {
        serde_json::from_slice::<Value>(&captured)
            .ok()
            .and_then(|v| v.get("usage").cloned())
    };
    let complete =
        status.is_success() && !truncated && response_complete(&captured, body["stream"] == true);
    let cutoff = complete && reasoning_cutoff(&captured, body["stream"] == true);
    let ollama_context = if complete && p.provider == Provider::Ollama {
        ollama_loaded_context(client, p, key).await
    } else {
        None
    };
    let generated = usage
        .as_ref()
        .and_then(|u| u["completion_tokens"].as_u64())
        .filter(|_| complete);
    let charged = generated.unwrap_or(allowance as u64);
    budget.0 = budget
        .0
        .saturating_add(allowance)
        .saturating_sub(charged.min(u32::MAX as u64) as u32);
    crate::config::atomic_write(
        &evidence.join(format!("{id}-receipt.json")),
        &serde_json::to_vec(
            &json!({"complete":complete,"status":status.as_u16(),"input_accounting":input,"response_sha256":crate::project::digest(&captured),"response_truncated":truncated,"request_sha256":crate::project::digest(&serde_json::to_vec(&body)?),"usage":usage,"generated_tokens":generated,"charged_generated_tokens":charged,"allowance":allowance,"provider_budget_violation":charged>allowance as u64,"thinking":thinking_sent,"reasoning_cutoff":cutoff,"ollama_loaded_context":ollama_context}),
        )?,
    )?;
    live_status.send_modify(|s| {
        if s.remaining_requests.is_some() {
            s.remaining_requests = Some(budget.1);
        }
        if s.remaining_generated_tokens.is_some() {
            s.remaining_generated_tokens = Some(budget.0);
        }
        if cutoff {
            s.reasoning_cutoffs += 1;
        }
        if ollama_context.is_some() {
            s.ollama_context = ollama_context;
        }
        s.stage = if !complete {
            Stage::Interrupted
        } else if budget.0 == 0 || budget.1 == 0 {
            Stage::AllowanceExhausted
        } else if cutoff {
            Stage::ReasoningCutoff
        } else {
            Stage::Ready
        };
    });
    writer.write_all(b"0\r\n\r\n").await?;
    Ok(())
}
