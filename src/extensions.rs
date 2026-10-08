//! Explicit external MCP connections. Tool calls still pass through Alt approval.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Transport {
    Stdio {
        command: PathBuf,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env_names: Vec<String>,
    },
    Http {
        url: String,
        auth_env: Option<String>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub name: String,
    pub transport: Transport,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub selected_tools: Vec<String>,
}
fn path(root: &Path, name: &str) -> Result<PathBuf> {
    ensure!(
        !name.is_empty()
            && name.len() <= 40
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c)),
        "Extension name must contain 1–40 letters, digits, underscores or hyphens"
    );
    Ok(root.join("extensions").join(format!("{name}.json")))
}
pub fn save(root: &Path, c: &Connection) -> Result<()> {
    match &c.transport {
        Transport::Stdio { command, .. } => {
            ensure!(!command.as_os_str().is_empty(), "Provide a server command")
        }
        Transport::Http { url, .. } => {
            let u = reqwest::Url::parse(url)?;
            ensure!(
                matches!(u.scheme(), "http" | "https")
                    && u.host_str().is_some()
                    && u.username().is_empty()
                    && u.password().is_none()
                    && u.fragment().is_none()
                    && u.query().is_none(),
                "Use an HTTP(S) MCP URL without embedded credentials, query or fragment"
            );
        }
    }
    ensure!(
        c.selected_tools.len() <= 16,
        "Choose up to 16 extension tools; use small relevant sets for small models"
    );
    crate::config::atomic_write(&path(root, &c.name)?, &serde_json::to_vec_pretty(c)?)
}
pub fn load(root: &Path, name: &str) -> Result<Connection> {
    Ok(serde_json::from_slice(&std::fs::read(path(root, name)?)?)?)
}
pub fn list(root: &Path) -> Result<Vec<Connection>> {
    let dir = root.join("extensions");
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut result = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let e = entry?;
        if e.path().extension().is_some_and(|x| x == "json") {
            let c: Connection = serde_json::from_slice(&std::fs::read(e.path())?)?;
            path(root, &c.name)?;
            result.push(c);
        }
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}
pub fn remove(root: &Path, name: &str) -> Result<()> {
    std::fs::remove_file(path(root, name)?)?;
    Ok(())
}
struct Process {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    group: crate::process::OwnedGroup,
}
impl Drop for Process {
    fn drop(&mut self) {
        self.group.kill();
        let _ = self.child.start_kill();
    }
}
enum Client {
    Stdio(Box<Process>),
    Http {
        client: reqwest::Client,
        url: String,
        key: Option<String>,
        session: Option<String>,
        protocol: String,
    },
}
/// Incremental SSE framing: dispatch only at a blank line, joining all data
/// fields. Network chunks are not UTF-8 or event boundaries. The caller bounds
/// the whole response to 1 MiB before feeding bytes here.
#[derive(Default)]
struct SseDecoder {
    line: Vec<u8>,
    data: String,
    after_cr: bool,
    first_line: bool,
}
impl SseDecoder {
    fn feed(&mut self, bytes: &[u8], id: u64) -> Result<Option<Value>> {
        for &byte in bytes {
            ensure!(
                self.line.len() + self.data.len() <= 1024 * 1024,
                "MCP event exceeds 1 MiB"
            );
            if self.after_cr && byte == b'\n' {
                self.after_cr = false;
                continue;
            }
            self.after_cr = byte == b'\r';
            if matches!(byte, b'\n' | b'\r') {
                let raw = std::mem::take(&mut self.line);
                let mut line = std::str::from_utf8(&raw).context("Invalid UTF-8 in MCP event")?;
                if !self.first_line {
                    line = line.strip_prefix('\u{feff}').unwrap_or(line);
                    self.first_line = true;
                }
                if line.is_empty() {
                    let data = std::mem::take(&mut self.data);
                    if !data.is_empty() {
                        let value: Value =
                            serde_json::from_str(&data).context("Unreadable MCP event data")?;
                        if value["id"] == json!(id) {
                            return Ok(Some(value));
                        }
                    }
                } else if let Some(value) = line.strip_prefix("data:") {
                    self.data.push_str(value.strip_prefix(' ').unwrap_or(value));
                    self.data.push('\n');
                } else if line == "data" {
                    self.data.push('\n');
                }
            } else {
                self.line.push(byte);
            }
        }
        Ok(None)
    }
}
impl Client {
    async fn connect(c: &Connection) -> Result<Self> {
        let mut client = match &c.transport {
            Transport::Stdio {
                command,
                args,
                env_names,
            } => {
                for name in env_names {
                    ensure!(
                        std::env::var_os(name).is_some(),
                        "Required server variable {name} is not set"
                    );
                }
                let mut command_line = Command::new(command);
                command_line
                    .args(args)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null());
                crate::process::configure(&mut command_line);
                let mut child = command_line
                    .spawn()
                    .context("Cannot start MCP server; check command and installed dependencies")?;
                let group = crate::process::OwnedGroup::capture(&child);
                let input = child.stdin.take().context("Server stdin")?;
                let output = BufReader::new(child.stdout.take().context("Server stdout")?);
                Self::Stdio(Box::new(Process {
                    child,
                    input,
                    output,
                    group,
                }))
            }
            Transport::Http { url, auth_env } => Self::Http {
                client: reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .connect_timeout(Duration::from_secs(10))
                    .build()?,
                url: url.clone(),
                key: auth_env
                    .as_ref()
                    .map(|name| {
                        std::env::var(name)
                            .with_context(|| format!("Authentication variable {name} is not set"))
                    })
                    .transpose()?,
                session: None,
                protocol: "2025-03-26".into(),
            },
        };
        let initialized=client.rpc(Some(1),"initialize",json!({"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"Alt","version":env!("CARGO_PKG_VERSION")}})).await?;
        ensure!(
            initialized["result"]["serverInfo"].is_object(),
            "Server did not return valid MCP initialization"
        );
        let version = initialized["result"]["protocolVersion"]
            .as_str()
            .context("MCP server omitted protocol version")?;
        ensure!(
            ["2024-11-05", "2025-03-26", "2025-06-18"].contains(&version),
            "Unsupported MCP protocol {version}"
        );
        if let Self::Http { protocol, .. } = &mut client {
            *protocol = version.into();
        }
        client
            .rpc(None, "notifications/initialized", json!({}))
            .await?;
        Ok(client)
    }
    async fn rpc(&mut self, id: Option<u64>, method: &str, params: Value) -> Result<Value> {
        let mut message = json!({"jsonrpc":"2.0","method":method,"params":params});
        if let Some(id) = id {
            message["id"] = json!(id);
        }
        let operation = async {
            match self {
                Self::Stdio(p) => {
                    let mut bytes = serde_json::to_vec(&message)?;
                    bytes.push(b'\n');
                    p.input.write_all(&bytes).await?;
                    p.input.flush().await?;
                    if id.is_none() {
                        return Ok(json!({}));
                    }
                    for _ in 0..100 {
                        let mut line = Vec::new();
                        (&mut p.output)
                            .take(1024 * 1024 + 1)
                            .read_until(b'\n', &mut line)
                            .await?;
                        ensure!(
                            !line.is_empty(),
                            "MCP server disconnected before returning a result"
                        );
                        ensure!(line.len() <= 1024 * 1024, "MCP response exceeds 1 MiB");
                        let value: Value = serde_json::from_slice(&line)
                            .context("MCP stdout must contain JSON-RPC, not log text")?;
                        if value["id"] == json!(id) {
                            return Ok(value);
                        }
                    }
                    bail!("Too many unrelated MCP notifications")
                }
                Self::Http {
                    client,
                    url,
                    key,
                    session,
                    protocol,
                } => {
                    let mut request = client
                        .post(url.as_str())
                        .header("Accept", "application/json, text/event-stream")
                        .header("MCP-Protocol-Version", protocol.as_str())
                        .json(&message);
                    if let Some(key) = key {
                        request = request.bearer_auth(key.as_str());
                    }
                    if let Some(session) = session.as_ref() {
                        request = request.header("Mcp-Session-Id", session);
                    }
                    let response = request.send().await?.error_for_status()?;
                    if let Some(id) = response.headers().get("Mcp-Session-Id") {
                        *session = Some(id.to_str()?.into());
                    }
                    if id.is_none() {
                        return Ok(json!({}));
                    }
                    let sse = response
                        .headers()
                        .get("content-type")
                        .is_some_and(|v| v.to_str().unwrap_or("").contains("text/event-stream"));
                    let mut bytes = Vec::new();
                    let mut events = SseDecoder::default();
                    let mut received = 0usize;
                    use futures_util::StreamExt;
                    let mut stream = response.bytes_stream();
                    while let Some(chunk) = stream.next().await {
                        let chunk = chunk?;
                        received += chunk.len();
                        ensure!(received <= 1024 * 1024, "MCP response exceeds 1 MiB");
                        if sse {
                            if let Some(value) = events.feed(&chunk, id.context("Request ID")?)? {
                                return Ok(value);
                            }
                        } else {
                            bytes.extend_from_slice(&chunk);
                        }
                    }
                    ensure!(
                        !sse,
                        "MCP event stream closed before a complete matching response"
                    );
                    let value: Value =
                        serde_json::from_slice(&bytes).context("Unreadable MCP HTTP response")?;
                    ensure!(
                        value["id"] == json!(id),
                        "MCP response ID does not match request"
                    );
                    Ok(value)
                }
            }
        };
        let value: Value = tokio::time::timeout(Duration::from_secs(120), operation)
            .await
            .context("MCP request timed out")??;
        ensure!(
            value.get("error").is_none(),
            "MCP error: {}",
            value["error"]
        );
        Ok(value)
    }
    async fn close(&mut self) {
        if let Self::Stdio(p) = self {
            let _ = p.input.shutdown().await;
            p.group.stop(&mut p.child).await;
        }
    }
}

#[cfg(test)]
mod sse_tests {
    use super::*;
    #[test]
    fn bounded_seeded_mutation_corpus_never_panics_or_grows_without_limit() {
        let seed = b"data:{\"id\":7,\"result\":{\"text\":\"hello\"}}\n\n";
        let mut random = 0x52414e444f4du64;
        for n in 0..8192 {
            let mut bytes = seed.to_vec();
            for _ in 0..1 + n % 12 {
                random ^= random << 13;
                random ^= random >> 7;
                random ^= random << 17;
                let i = random as usize % bytes.len();
                bytes[i] = (random >> 32) as u8;
            }
            let mut parser = SseDecoder::default();
            for part in bytes.chunks(1 + n % 17) {
                if parser.feed(part, 7).is_err() {
                    break;
                }
            }
            assert!(parser.line.len() + parser.data.len() <= 1024 * 1024);
        }
        assert!(
            SseDecoder::default()
                .feed(&vec![b'x'; 1024 * 1024 + 2], 7)
                .is_err()
        );
    }
    #[test]
    fn framing_survives_every_chunk_boundary_and_requires_event_termination() {
        for ending in ["\n", "\r\n", "\r"] {
            let wire = format!(
                "\u{feff}: heartbeat{ending}{ending}data:{{\"id\":99}}{ending}{ending}event: message{ending}data:{{\"id\":7,{ending}data: \"result\":{{\"text\":\"café 🙂\"}}}}{ending}{ending}"
            );
            for split in 0..=wire.len() {
                let mut decoder = SseDecoder::default();
                let first = decoder.feed(&wire.as_bytes()[..split], 7).unwrap();
                let value = first.or_else(|| decoder.feed(&wire.as_bytes()[split..], 7).unwrap());
                assert_eq!(value.unwrap()["result"]["text"], "café 🙂");
            }
            let mut decoder = SseDecoder::default();
            let mut found = None;
            for byte in wire.bytes() {
                found = found.or(decoder.feed(&[byte], 7).unwrap());
            }
            assert!(found.is_some());
        }
        let mut decoder = SseDecoder::default();
        assert!(
            decoder
                .feed(b"data:{\"id\":7,\"result\":{}}\n", 7)
                .unwrap()
                .is_none()
        );
        assert!(decoder.feed(b"\n", 7).unwrap().is_some());
    }
}
pub async fn probe(root: &Path, name: &str) -> Result<Value> {
    probe_with_policy(
        root,
        name,
        crate::config::Preferences::load(root)?.access_policy,
    )
    .await
}
pub fn require_full_access(policy: crate::project::Policy) -> Result<()> {
    ensure!(
        policy == crate::project::Policy::Trusted,
        "External extensions require Full access; no server was started or contacted"
    );
    Ok(())
}
pub async fn probe_with_policy(
    root: &Path,
    name: &str,
    policy: crate::project::Policy,
) -> Result<Value> {
    require_full_access(policy)?;
    let c = load(root, name)?;
    let mut client = Client::connect(&c).await?;
    let result: Result<Value> = async {
        let mut all = Vec::new();
        let mut cursor = None;
        for id in 2..34 {
            let params = cursor
                .as_ref()
                .map(|c: &String| json!({"cursor":c}))
                .unwrap_or(json!({}));
            let page = client.rpc(Some(id), "tools/list", params).await?;
            all.extend(
                page["result"]["tools"]
                    .as_array()
                    .context("Invalid tool inventory")?
                    .clone(),
            );
            ensure!(all.len() <= 512, "MCP inventory exceeds 512 tools");
            cursor = page["result"]["nextCursor"].as_str().map(str::to_owned);
            if cursor.is_none() {
                return Ok(json!({"result":{"tools":all}}));
            }
        }
        bail!("MCP inventory pagination did not finish")
    }
    .await;
    client.close().await;
    let result = result?;
    ensure!(
        result["result"]["tools"].is_array(),
        "Server did not provide a tool inventory"
    );
    let inventory = result["result"].clone();
    crate::config::atomic_write(
        &root.join("extensions/cache").join(format!("{name}.json")),
        &serde_json::to_vec_pretty(&inventory)?,
    )?;
    Ok(inventory)
}
pub fn select(root: &Path, name: &str, tools: Vec<String>) -> Result<()> {
    let mut c = load(root, name)?;
    let cached: Value = serde_json::from_slice(&std::fs::read(
        root.join("extensions/cache").join(format!("{name}.json")),
    )?)?;
    for tool in &tools {
        ensure!(
            cached["tools"]
                .as_array()
                .context("Check the connection first")?
                .iter()
                .any(|t| t["name"] == *tool),
            "Unknown tool {tool}; check the connection again"
        );
    }
    c.enabled = !tools.is_empty();
    c.selected_tools = tools;
    save(root, &c)
}
pub fn selected_inventory(root: &Path, name: &str) -> Result<Value> {
    let c = load(root, name)?;
    ensure!(c.enabled, "Extension is disabled");
    let mut inventory: Value = serde_json::from_slice(&std::fs::read(
        root.join("extensions/cache").join(format!("{name}.json")),
    )?)?;
    let tools = inventory["tools"]
        .as_array_mut()
        .context("Check extension health to refresh its inventory")?;
    tools.retain(|t| c.selected_tools.iter().any(|s| t["name"] == *s));
    for tool in tools {
        tool["annotations"] = json!({"readOnlyHint":true,"openWorldHint":true});
    }
    Ok(inventory)
}
/// Connections persist for a conversation. Failures discard the connection;
/// the next explicit call reconnects, without replaying a possibly completed action.
#[derive(Default)]
pub struct Manager {
    clients: tokio::sync::Mutex<std::collections::HashMap<String, (String, Client)>>,
}
impl Manager {
    pub async fn call(
        &self,
        root: &Path,
        name: &str,
        tool: &str,
        arguments: Value,
    ) -> Result<Value> {
        self.call_with_policy(
            root,
            name,
            tool,
            arguments,
            crate::config::Preferences::load(root)?.access_policy,
        )
        .await
    }
    pub async fn call_with_policy(
        &self,
        root: &Path,
        name: &str,
        tool: &str,
        arguments: Value,
        policy: crate::project::Policy,
    ) -> Result<Value> {
        require_full_access(policy)?;
        let c = load(root, name)?;
        ensure!(
            c.enabled && c.selected_tools.iter().any(|t| t == tool),
            "This external tool is not selected"
        );
        let signature = serde_json::to_string(&c)?;
        let mut clients = self.clients.lock().await;
        if clients.get(name).is_some_and(|(old, _)| old != &signature) {
            clients.remove(name);
        }
        if !clients.contains_key(name) {
            clients.insert(name.into(), (signature, Client::connect(&c).await?));
        }
        let result = clients
            .get_mut(name)
            .expect("connected")
            .1
            .rpc(
                Some(3),
                "tools/call",
                json!({"name":tool,"arguments":arguments}),
            )
            .await;
        let value = match result {
            Ok(v) => v,
            Err(e) => {
                clients.remove(name);
                return Err(e.context(
                    "External call failed. It was not replayed; check side effects before retrying",
                ));
            }
        };
        ensure!(
            value["result"]["content"].is_array(),
            "External tool returned no MCP content"
        );
        Ok(value["result"].clone())
    }
}
pub async fn call(root: &Path, name: &str, tool: &str, arguments: Value) -> Result<Value> {
    Manager::default().call(root, name, tool, arguments).await
}
pub async fn proxy(root: &Path, name: &str, socket: &Path) -> Result<()> {
    let mut reader = BufReader::new(tokio::io::stdin());
    let mut writer = tokio::io::stdout();
    loop {
        let mut line = Vec::new();
        (&mut reader)
            .take(1024 * 1024 + 1)
            .read_until(b'\n', &mut line)
            .await?;
        if line.is_empty() {
            break;
        }
        ensure!(line.len() <= 1024 * 1024, "MCP request exceeds 1 MiB");
        let request: Value = serde_json::from_slice(&line)?;
        let Some(id) = request.get("id") else {
            continue;
        };
        let result:Result<Value>=async {match request["method"].as_str().unwrap_or("") {
   "initialize"=>Ok(json!({"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":name,"version":env!("CARGO_PKG_VERSION")}})),
   "ping"=>Ok(json!({})),"tools/list"=>selected_inventory(root,name),
   "tools/call"=>{let mut stream=tokio::net::UnixStream::connect(socket).await?;let mut data=serde_json::to_vec(&json!({"name":"extension","arguments":{"connection":name,"tool":request["params"]["name"],"arguments":request["params"]["arguments"]}}))?;data.push(b'\n');stream.write_all(&data).await?;let mut bytes=Vec::new();stream.take(1024*1024).read_to_end(&mut bytes).await?;Ok(serde_json::from_slice(&bytes)?)},
   method=>bail!("Unsupported method {method}")
  }}.await;
        let mut response = match result {
            Ok(v) => json!({"jsonrpc":"2.0","id":id,"result":v}),
            Err(e) => {
                json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":format!("{e:#}")}})
            }
        }
        .to_string();
        response.push('\n');
        writer.write_all(response.as_bytes()).await?;
        writer.flush().await?;
    }
    Ok(())
}
