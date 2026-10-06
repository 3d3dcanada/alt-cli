//! Optional LSP references and reviewable rename. No server is silently installed.
use crate::project::{Policy, Project};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{path::Path, process::Stdio, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
pub struct Query<'a> {
    pub server: &'a Path,
    pub args: &'a [String],
    pub path: &'a str,
    pub line: u32,
    pub column: u32,
    pub new_name: Option<&'a str>,
}
struct Client {
    child: tokio::process::Child,
    input: tokio::process::ChildStdin,
    output: BufReader<tokio::process::ChildStdout>,
    identity: Option<crate::jobs::Identity>,
    seq: u64,
}
impl Drop for Client {
    fn drop(&mut self) {
        if let Some(id) = &self.identity
            && id.alive()
        {
            let _ = nix::sys::signal::killpg(
                nix::unistd::Pid::from_raw(id.pid as i32),
                nix::sys::signal::Signal::SIGKILL,
            );
        }
        let _ = self.child.start_kill();
    }
}
impl Client {
    async fn send(&mut self, v: Value) -> Result<()> {
        let bytes = serde_json::to_vec(&v)?;
        ensure!(bytes.len() <= 8 * 1024 * 1024, "LSP request too large");
        self.input
            .write_all(format!("Content-Length: {}\r\n\r\n", bytes.len()).as_bytes())
            .await?;
        self.input.write_all(&bytes).await?;
        self.input.flush().await?;
        Ok(())
    }
    async fn read(&mut self) -> Result<Value> {
        let mut length = None;
        for _ in 0..32 {
            let mut line = String::new();
            let n = (&mut self.output).take(8193).read_line(&mut line).await?;
            ensure!(
                n > 0 && n <= 8192,
                "Invalid LSP header or disconnected server"
            );
            if line == "\r\n" || line == "\n" {
                break;
            }
            if let Some((k, v)) = line.split_once(':')
                && k.eq_ignore_ascii_case("content-length")
            {
                length = Some(v.trim().parse::<usize>()?);
            }
        }
        let n = length.context("LSP message omitted Content-Length")?;
        ensure!(n <= 8 * 1024 * 1024, "LSP response exceeds 8 MiB");
        let mut bytes = vec![0; n];
        self.output.read_exact(&mut bytes).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }
    async fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        self.seq += 1;
        let id = self.seq;
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
            .await?;
        tokio::time::timeout(Duration::from_secs(60), async {
            for _ in 0..1000 {
                let v = self.read().await?;
                if v.get("method").is_none() && v["id"] == id {
                    ensure!(
                        v.get("error").is_none(),
                        "Language server error: {}",
                        v["error"]
                    );
                    return Ok(v["result"].clone());
                }
                if v.get("method").is_some() && v.get("id").is_some() {
                    let result = match v["method"].as_str().unwrap_or("") {
                        "workspace/configuration" => json!(
                            v["params"]["items"]
                                .as_array()
                                .map(|a| vec![Value::Null; a.len()])
                                .unwrap_or_default()
                        ),
                        _ => Value::Null,
                    };
                    self.send(json!({"jsonrpc":"2.0","id":v["id"],"result":result}))
                        .await?;
                }
            }
            bail!("Too many unrelated language server messages")
        })
        .await
        .context("Language server timed out; no edits were applied")?
    }
}
fn uri(path: &Path) -> Result<String> {
    Ok(reqwest::Url::from_file_path(path)
        .map_err(|_| anyhow::anyhow!("Invalid file URI"))?
        .into())
}
/// Protocol columns are UTF-16; reject invalid offsets instead of splitting text.
pub fn offset(text: &str, line: u64, column: u64) -> Result<usize> {
    let mut start = 0;
    for _ in 0..line {
        start += text[start..]
            .find('\n')
            .context("LSP line outside document")?
            + 1;
    }
    let tail = &text[start..];
    let mut units = 0;
    for (i, c) in tail.char_indices() {
        if units == column {
            return Ok(start + i);
        }
        ensure!(c != '\n', "LSP column outside line");
        units += c.len_utf16() as u64;
        ensure!(units <= column, "LSP offset splits a surrogate pair");
    }
    ensure!(units == column, "LSP column outside document");
    Ok(text.len())
}
pub async fn query(data: &Path, cwd: &Path, q: Query<'_>, policy: Policy) -> Result<Value> {
    ensure!(
        policy == Policy::Trusted,
        "Language servers execute local tools. Choose Full access"
    );
    Project::validate_path(q.path)?;
    ensure!(
        q.line > 0 && q.column > 0,
        "Line and column start at 1 (columns count UTF-16 units)"
    );
    let cwd = cwd.canonicalize()?;
    let p = Project::open(data, &cwd)?;
    let task = format!("lsp-{}", uuid::Uuid::new_v4());
    p.start_task(&task, "Language-server navigation or rename")?;
    let read = p.read(&task, q.path, 1, 1)?;
    let _ = read;
    let text = p.dir.read_to_string(q.path)?;
    let source = if q.new_name.is_some() {
        Some(p.snapshot()?.1)
    } else {
        None
    };
    drop(p);
    let mut command = tokio::process::Command::new(q.server);
    command
        .args(q.args)
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    crate::process::configure(&mut command);
    let mut child = command
        .spawn()
        .context("Cannot start language server; install it or correct the executable")?;
    let identity = child.id().and_then(crate::jobs::Identity::read);
    let input = child.stdin.take().context("LSP input")?;
    let output = BufReader::new(child.stdout.take().context("LSP output")?);
    let mut client = Client {
        child,
        input,
        output,
        identity,
        seq: 0,
    };
    let initialize=client.request("initialize",json!({"processId":std::process::id(),"rootUri":uri(&cwd)?,"workspaceFolders":[{"uri":uri(&cwd)?,"name":"project"}],"capabilities":{"general":{"positionEncodings":["utf-16"]},"workspace":{"configuration":true,"workspaceEdit":{"documentChanges":true}},"textDocument":{"rename":{"prepareSupport":false}}}})).await?;
    ensure!(
        initialize["capabilities"]["positionEncoding"]
            .as_str()
            .is_none_or(|e| e == "utf-16"),
        "Server chose an unsupported position encoding"
    );
    client
        .send(json!({"jsonrpc":"2.0","method":"initialized","params":{}}))
        .await?;
    let file_uri = uri(&cwd.join(q.path))?;
    let language = match Path::new(q.path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
    {
        "rs" => "rust",
        "py" => "python",
        "js" | "mjs" => "javascript",
        _ => "plaintext",
    };
    client.send(json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":file_uri,"languageId":language,"version":1,"text":text}}})).await?;
    let mut params = json!({"textDocument":{"uri":file_uri},"position":{"line":q.line-1,"character":q.column-1},"context":{"includeDeclaration":true}});
    let method = if let Some(name) = q.new_name {
        ensure!(!name.trim().is_empty(), "Enter a new identifier");
        params["newName"] = json!(name);
        "textDocument/rename"
    } else {
        "textDocument/references"
    };
    // Servers may acknowledge initialize before workspace loading completes.
    // These requests only compute locations/edits; retrying does not apply edits.
    let mut result = client.request(method, params.clone()).await;
    for _ in 0..20 {
        let loading = match &result {
            Ok(v) => v.is_null() || v.as_array().is_some_and(|a| a.is_empty()),
            Err(e) => {
                let m = e.to_string();
                m.contains("No references found")
                    || m.contains("content modified")
                    || m.contains("not ready")
            }
        };
        if !loading {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
        result = client.request(method, params.clone()).await;
    }
    let result = result?;
    if q.new_name.is_none() {
        return Ok(json!({"references":result,"server":q.server,"position_encoding":"UTF-16"}));
    }
    let p = Project::open(data, &cwd)?;
    ensure!(
        source.as_ref() == Some(&p.snapshot()?.1),
        "Project changed while the language server was computing edits; retry from current files"
    );
    drop(p);
    let changes = prepare_observed_edit(data, &cwd, &task, &result, source.as_ref())?;
    Ok(
        json!({"task":task,"changes":changes,"applied":false,"note":"Review diffs. Applying a rename uses Alt checkpoints and current-file hash checks."}),
    )
}
pub fn prepare_workspace_edit(
    data: &Path,
    cwd: &Path,
    task: &str,
    edit: &Value,
) -> Result<Vec<crate::project::Change>> {
    prepare_observed_edit(data, cwd, task, edit, None)
}
fn prepare_observed_edit(
    data: &Path,
    cwd: &Path,
    task: &str,
    edit: &Value,
    observed: Option<&std::collections::BTreeMap<String, Vec<u8>>>,
) -> Result<Vec<crate::project::Change>> {
    let mut documents = std::collections::BTreeMap::<String, Vec<Value>>::new();
    if let Some(changes) = edit["changes"].as_object() {
        for (uri, edits) in changes {
            documents
                .entry(uri.clone())
                .or_default()
                .extend(edits.as_array().context("Invalid LSP edits")?.clone());
        }
    }
    if let Some(changes) = edit["documentChanges"].as_array() {
        for c in changes {
            ensure!(
                c.get("kind").is_none(),
                "File create/rename/delete operations need separate review"
            );
            let uri = c["textDocument"]["uri"]
                .as_str()
                .context("Missing document URI")?;
            documents
                .entry(uri.into())
                .or_default()
                .extend(c["edits"].as_array().context("Missing text edits")?.clone());
        }
    }
    ensure!(
        !documents.is_empty(),
        "Language server returned no rename edits"
    );
    ensure!(
        documents.len() <= 128,
        "Rename exceeds 128 files; split this change"
    );
    let p = Project::open(data, cwd)?;
    p.note(
        task,
        "plan",
        "Review and apply language-server identifier rename",
        "user",
    )?;
    let mut prepared = Vec::new();
    for (uri, edits) in documents {
        let absolute = reqwest::Url::parse(&uri)?
            .to_file_path()
            .map_err(|_| anyhow::anyhow!("LSP edit is not a local file"))?;
        let relative = absolute
            .strip_prefix(&p.root)
            .context("Language server tried to edit outside this project")?;
        let path = relative.to_str().context("Non-UTF8 path")?;
        p.read(task, path, 1, 1)?;
        let before = p.dir.read_to_string(path)?;
        if let Some(observed) = observed {
            ensure!(
                observed.get(path).map(Vec::as_slice) == Some(before.as_bytes()),
                "File {path} changed after language-server analysis; recompute the rename"
            );
        }
        let mut ranges = Vec::new();
        for e in edits {
            let r = &e["range"];
            let a = offset(
                &before,
                r["start"]["line"].as_u64().context("Line")?,
                r["start"]["character"].as_u64().context("Column")?,
            )?;
            let b = offset(
                &before,
                r["end"]["line"].as_u64().context("Line")?,
                r["end"]["character"].as_u64().context("Column")?,
            )?;
            ensure!(a <= b, "Invalid LSP range");
            ranges.push((
                a,
                b,
                e["newText"]
                    .as_str()
                    .context("Missing replacement")?
                    .to_owned(),
            ));
        }
        ranges.sort_by_key(|r| r.0);
        ensure!(
            ranges.windows(2).all(|r| r[0].1 <= r[1].0),
            "Overlapping LSP edits"
        );
        let mut after = before.clone();
        for (a, b, text) in ranges.into_iter().rev() {
            after.replace_range(a..b, &text);
        }
        prepared.push(p.prepare_edit(
            task,
            path,
            None,
            &before,
            &after,
            "replace",
            "Language-server rename",
        )?);
    }
    Ok(prepared)
}
pub fn apply(data: &Path, cwd: &Path, ids: &[String], policy: Policy) -> Result<Value> {
    let p = Project::open(data, cwd)?;
    let mut changes = Vec::new();
    for id in ids {
        let c = p.change(id)?;
        ensure!(c.status == "proposed", "Rename proposal already handled");
        ensure!(
            p.dir
                .read(&c.path)
                .ok()
                .as_deref()
                .map(crate::project::digest)
                == c.before_hash,
            "Rename conflicts with current file {}; no changes applied",
            c.path
        );
        changes.push(c);
    }
    let mut done = Vec::new();
    for c in changes {
        done.push(p.apply(&c.id, policy)?);
    }
    Ok(json!({"applied":done}))
}
