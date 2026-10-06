//! Alt-owned PTY jobs. A private supervisor outlives a conversation only when selected.
use anyhow::{Context, Result, bail, ensure};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
};
const LOG_LIMIT: usize = 8 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identity {
    pub pid: u32,
    pub start: String,
    pub boot: String,
}
impl Identity {
    pub fn read(pid: u32) -> Option<Self> {
        let raw = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let fields: Vec<_> = raw.rsplit_once(") ")?.1.split_whitespace().collect();
        if fields.first() == Some(&"Z") {
            return None;
        }
        Some(Self {
            pid,
            start: fields.get(19)?.to_string(),
            boot: std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
                .ok()?
                .trim()
                .into(),
        })
    }
    pub fn alive(&self) -> bool {
        Self::read(self.pid).as_ref() == Some(self)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub name: String,
    pub command: String,
    pub cwd: PathBuf,
    pub keep: bool,
    pub timeout_secs: u64,
    pub rows: u16,
    pub cols: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub spec: Spec,
    pub status: String,
    pub owner: Option<Identity>,
    pub supervisor: Option<Identity>,
    pub process: Option<Identity>,
    pub exit_code: Option<u32>,
    pub started: u64,
    pub output_bytes: u64,
    pub output_truncated: bool,
    pub error: Option<String>,
}
fn directory(root: &Path, id: &str) -> Result<PathBuf> {
    uuid::Uuid::parse_str(id).context("Invalid job ID")?;
    Ok(root.join("jobs").join(id))
}
fn socket(root: &Path, id: &str) -> Result<PathBuf> {
    Ok(directory(root, id)?.join("control.sock"))
}
fn save(root: &Path, r: &Record) -> Result<()> {
    crate::config::atomic_write(
        &directory(root, &r.id)?.join("job.json"),
        &serde_json::to_vec_pretty(r)?,
    )
}
pub fn record(root: &Path, id: &str) -> Result<Record> {
    let mut r: Record =
        serde_json::from_slice(&std::fs::read(directory(root, id)?.join("job.json"))?)?;
    if matches!(r.status.as_str(), "running" | "starting")
        && r.supervisor.as_ref().is_some_and(|i| !i.alive())
    {
        r.status = "interrupted".into();
        r.error=Some("Supervisor is no longer the recorded process. No PID was signalled; inspect saved output or restart.".into());
    }
    Ok(r)
}
pub fn list(root: &Path) -> Result<Vec<Record>> {
    let mut jobs = Vec::new();
    if !root.join("jobs").exists() {
        return Ok(jobs);
    }
    for e in std::fs::read_dir(root.join("jobs"))? {
        let e = e?;
        if let Ok(r) = record(root, &e.file_name().to_string_lossy()) {
            jobs.push(r);
        }
    }
    jobs.sort_by_key(|r| std::cmp::Reverse(r.started));
    Ok(jobs)
}
pub async fn health(root: &Path, id: &str, url: &str) -> Result<Value> {
    let r = record(root, id)?;
    ensure!(r.status == "running", "Job is {}", r.status);
    let url = reqwest::Url::parse(url).context("Enter an HTTP(S) health URL")?;
    ensure!(
        matches!(url.scheme(), "http" | "https")
            && url.username().is_empty()
            && url.password().is_none(),
        "Use an HTTP(S) URL without embedded credentials"
    );
    let response = crate::models::client()?
        .get(url.clone())
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    Ok(
        json!({"job":id,"url":url.as_str(),"status":response.status().as_u16(),"healthy":response.status().is_success(),"scope":"Service response only; not complete application verification"}),
    )
}
pub async fn start(root: &Path, mut spec: Spec) -> Result<Record> {
    ensure!(
        !spec.command.trim().is_empty() && spec.command.len() <= 64_000,
        "Enter a command up to 64 KB"
    );
    ensure!(
        !spec.name.trim().is_empty() && spec.name.len() <= 120,
        "Use a job name up to 120 bytes"
    );
    ensure!(
        (2..=200).contains(&spec.rows) && (10..=400).contains(&spec.cols),
        "Terminal size must be 10–400 columns and 2–200 rows"
    );
    spec.cwd = spec.cwd.canonicalize()?;
    ensure!(spec.cwd.is_dir(), "Choose a project folder");
    let id = uuid::Uuid::new_v4().to_string();
    let dir = directory(root, &id)?;
    crate::config::private_dir(&dir)?;
    ensure!(
        socket(root, &id)?.as_os_str().len() < 104,
        "Job state path is too long for a Unix socket; choose a shorter Alt data directory"
    );
    let r = Record {
        id: id.clone(),
        owner: if spec.keep {
            None
        } else {
            Identity::read(std::process::id())
        },
        spec,
        status: "starting".into(),
        supervisor: None,
        process: None,
        exit_code: None,
        started: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        output_bytes: 0,
        output_truncated: false,
        error: None,
    };
    save(root, &r)?;
    let mut command = std::process::Command::new(std::env::current_exe()?);
    command
        .args(["--data-dir"])
        .arg(root)
        .args(["job-worker", "--id", &id])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(std::fs::File::create(
            dir.join("supervisor.log"),
        )?));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .context("Could not start the job supervisor")?;
    // Reap when the UI lives long enough; an explicitly persistent job may outlive it.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    for _ in 0..100 {
        let r = record(root, &id)?;
        if r.status != "starting" {
            return Ok(r);
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    bail!("Supervisor did not become ready; job {id} retains its startup log")
}
pub async fn request(root: &Path, id: &str, value: Value) -> Result<Value> {
    let r = record(root, id)?;
    ensure!(
        r.status == "running",
        "Job is {}. Its saved output is available; restart to run it again",
        r.status
    );
    let operation = async {
        let mut stream = UnixStream::connect(socket(root, id)?).await?;
        let mut message = serde_json::to_vec(&value)?;
        ensure!(message.len() <= 32 * 1024, "Job request too large");
        message.push(b'\n');
        stream.write_all(&message).await?;
        let mut response = Vec::new();
        stream.take(256 * 1024).read_to_end(&mut response).await?;
        let result: Value = serde_json::from_slice(&response)?;
        ensure!(result.get("error").is_none(), "{}", result["error"]);
        Ok(result)
    };
    tokio::time::timeout(Duration::from_secs(3), operation)
        .await
        .context("Job supervisor did not respond; its record and log are preserved")?
}
pub async fn stop(root: &Path, id: &str) -> Result<Record> {
    let r = record(root, id)?;
    if r.status != "running" {
        return Ok(r);
    }
    request(root, id, json!({"action":"stop"})).await?;
    for _ in 0..60 {
        let r = record(root, id)?;
        if r.status != "running" {
            return Ok(r);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    bail!("Job stop has not completed; inspect its saved record")
}
pub async fn restart(root: &Path, id: &str) -> Result<Record> {
    let r = stop(root, id).await?;
    start(root, r.spec).await
}
pub fn output(root: &Path, id: &str) -> Result<String> {
    Ok(crate::display_text(&String::from_utf8_lossy(
        &std::fs::read(directory(root, id)?.join("output.log"))?,
    )))
}
pub async fn stop_owned(root: &Path) -> Result<()> {
    let owner = Identity::read(std::process::id());
    for r in list(root)? {
        if !r.spec.keep && r.owner == owner && r.status == "running" {
            let _ = stop(root, &r.id).await;
        }
    }
    Ok(())
}
fn kill_group(r: &Record) {
    if let Some(process) = &r.process {
        // Parent identity prevents signalling a reused PID. PTY's session leader has
        // the process group ID recorded here; descendants are cleaned up on normal exit.
        let mut owns = process.alive();
        if !owns && let Ok(entries) = std::fs::read_dir("/proc") {
            for e in entries.flatten() {
                let Ok(pid) = e.file_name().to_string_lossy().parse::<u32>() else {
                    continue;
                };
                let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
                    continue;
                };
                let Some((_, fields)) = stat.rsplit_once(") ") else {
                    continue;
                };
                if fields
                    .split_whitespace()
                    .nth(2)
                    .and_then(|s| s.parse::<u32>().ok())
                    == Some(process.pid)
                    && std::fs::read(format!("/proc/{pid}/environ")).is_ok_and(|v| {
                        v.split(|b| *b == 0)
                            .any(|s| s == format!("ALT_JOB_ID={}", r.id).as_bytes())
                    })
                {
                    owns = true;
                    break;
                }
            }
        }
        if owns {
            let _ = nix::sys::signal::killpg(
                nix::unistd::Pid::from_raw(process.pid as i32),
                nix::sys::signal::Signal::SIGKILL,
            );
        }
    }
}
struct Cleanup(Record);
impl Drop for Cleanup {
    fn drop(&mut self) {
        kill_group(&self.0);
    }
}
async fn serve(
    mut stream: UnixStream,
    record: &Record,
    screen: &vt100::Parser,
    tail: &VecDeque<u8>,
    offset: u64,
) -> Result<(Value, UnixStream)> {
    let mut bytes = Vec::new();
    let mut reader = BufReader::new(&mut stream);
    tokio::time::timeout(
        Duration::from_secs(2),
        (&mut reader)
            .take(32 * 1024 + 1)
            .read_until(b'\n', &mut bytes),
    )
    .await??;
    ensure!(bytes.len() <= 32 * 1024, "Job request too large");
    let mut v: Value = serde_json::from_slice(&bytes)?;
    if v["action"] == "snapshot" {
        v = json!({"record":record,"screen":screen.screen().contents()});
    } else if v["action"] == "output" {
        let from = v["offset"].as_u64().unwrap_or(0);
        let skip = from.saturating_sub(offset).min(tail.len() as u64) as usize;
        let bytes: Vec<u8> = tail.iter().skip(skip).take(16 * 1024).copied().collect();
        v = json!({"bytes":bytes,"offset":offset+skip as u64+bytes.len() as u64,"truncated":from<offset});
    }
    Ok((v, stream))
}
pub async fn worker(root: &Path, id: &str) -> Result<()> {
    let mut r = record(root, id)?;
    r.supervisor = Identity::read(std::process::id());
    save(root, &r)?;
    let result = worker_inner(root, &mut r).await;
    if let Err(e) = &result {
        r.status = "failed".into();
        r.error = Some(format!("{e:#}"));
        save(root, &r)?;
    }
    let _ = std::fs::remove_file(socket(root, id)?);
    result
}
async fn worker_inner(root: &Path, r: &mut Record) -> Result<()> {
    let path = socket(root, &r.id)?;
    let listener = UnixListener::bind(&path)?;
    let pair = native_pty_system().openpty(PtySize {
        rows: r.spec.rows,
        cols: r.spec.cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;
    let mut command = CommandBuilder::new("/bin/bash");
    command.args(["-c", &r.spec.command]);
    command.cwd(&r.spec.cwd);
    command.env("TERM", "xterm-256color");
    command.env("ALT_JOB_ID", &r.id);
    let mut child = pair.slave.spawn_command(command)?;
    drop(pair.slave);
    r.process = child.process_id().and_then(Identity::read);
    let guard = Cleanup(r.clone());
    let mut writer = pair.master.take_writer()?;
    // PTY input can block when the child stops reading. Keep the supervisor's
    // stop/resize/health loop independent, and bound queued input to 256 KiB.
    let (input_tx, input_rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(32);
    std::thread::spawn(move || {
        while let Ok(bytes) = input_rx.recv() {
            if writer
                .write_all(&bytes)
                .and_then(|_| writer.flush())
                .is_err()
            {
                break;
            }
        }
    });
    let mut reader = pair.master.try_clone_reader()?;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(32);
    std::thread::spawn(move || {
        let mut buf = [0; 8192];
        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 || tx.blocking_send(buf[..n].to_vec()).is_err() {
                break;
            }
        }
    });
    let mut parser = vt100::Parser::new(r.spec.rows, r.spec.cols, 2000);
    let mut tail = VecDeque::new();
    let mut offset = 0u64;
    let mut log = std::fs::File::create(directory(root, &r.id)?.join("output.log"))?;
    r.status = "running".into();
    save(root, r)?;
    let started = Instant::now();
    let mut tick = tokio::time::interval(Duration::from_millis(50));
    let shutdown = crate::process::shutdown_signal();
    tokio::pin!(shutdown);
    let mut stop_reason = None;
    let mut ended = None;
    let mut drain_until = None;
    let mut output_open = true;
    loop {
        tokio::select! {
         _=&mut shutdown=>{stop_reason=Some("stopped");break;},
         bytes=rx.recv(),if output_open=>{if let Some(bytes)=bytes {
          parser.process(&bytes);r.output_bytes+=bytes.len() as u64;
          let used=log.metadata()?.len() as usize;let n=bytes.len().min(LOG_LIMIT.saturating_sub(used));log.write_all(&bytes[..n])?;r.output_truncated|=n<bytes.len();
          tail.extend(bytes);while tail.len()>128*1024{tail.pop_front();offset+=1;}
         }else {output_open=false;if ended.is_some(){break;}}},
         accepted=listener.accept()=>{
          let (stream,_)=accepted?;
          if let Ok((v,mut stream))=serve(stream,r,&parser,&tail,offset).await {
            let mut response=v.clone();
            match v["action"].as_str() {
             Some("input")=>{let text=v["text"].as_str().unwrap_or("");response=if text.len()>8192 {json!({"error":"Input exceeds 8 KB"})}else{match input_tx.try_send(text.as_bytes().to_vec()){Ok(())=>json!({"queued":true}),Err(_)=>json!({"error":"Terminal input is full or closed; inspect the job before sending more"})}};},
             Some("resize")=>{
              let rows=v["rows"].as_u64().unwrap_or(24).clamp(2,200) as u16;let cols=v["cols"].as_u64().unwrap_or(80).clamp(10,400) as u16;
              pair.master.resize(PtySize{rows,cols,pixel_width:0,pixel_height:0})?;parser.set_size(rows,cols);r.spec.rows=rows;r.spec.cols=cols;response=json!({"resized":true});
             },
             Some("stop")=>{stop_reason=Some("stopped");response=json!({"stopping":true});},
             Some(_)=>{response=json!({"error":"Unknown job action"});},
             None=>{}
            }
            // A caller may detach or disappear after sending a request. Its
            // transport failure must not terminate the supervised program, and
            // a caller that stops reading must not block Stop indefinitely.
            let bytes=serde_json::to_vec(&response)?;
            let _=tokio::time::timeout(Duration::from_millis(250),async {
                stream.write_all(&bytes).await?;
                stream.shutdown().await
            }).await;
            if stop_reason.is_some(){break;}
          }
         },
         _=tick.tick()=>{
           if r.owner.as_ref().is_some_and(|o|!o.alive()){stop_reason=Some("owner exited");break;}
           if r.spec.timeout_secs>0&&started.elapsed().as_secs()>=r.spec.timeout_secs{stop_reason=Some("timed out");break;}
           if ended.is_none()&&let Some(status)=child.try_wait()?{ended=Some(status.exit_code());kill_group(r);drain_until=Some(Instant::now()+Duration::from_millis(300));}
           if drain_until.is_some_and(|d|Instant::now()>=d){break;}
         }
        }
    }
    drop(guard);
    if ended.is_none() {
        let _ = child.kill();
        let _ = child.wait();
    }
    log.sync_all()?;
    crate::config::atomic_write(
        &directory(root, &r.id)?.join("tail.json"),
        &serde_json::to_vec(&json!({"start":offset,"end":offset+tail.len() as u64,"bytes":tail}))?,
    )?;
    r.exit_code = ended;
    r.status = stop_reason
        .unwrap_or(if ended == Some(0) {
            "completed"
        } else {
            "failed"
        })
        .into();
    save(root, r)?;
    Ok(())
}
/// Standalone terminal attachment; Ctrl+] detaches without terminating the job.
pub async fn attach(root: &Path, id: &str) -> Result<()> {
    use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers};
    use futures_util::StreamExt;
    ensure!(
        std::io::IsTerminal::is_terminal(&std::io::stdin()),
        "Attach requires an interactive terminal"
    );
    crossterm::terminal::enable_raw_mode()?;
    struct Restore;
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = crossterm::terminal::disable_raw_mode();
        }
    }
    let _restore = Restore;
    let mut events = EventStream::new();
    let mut tick = tokio::time::interval(Duration::from_millis(40));
    let mut offset = 0u64;
    loop {
        tokio::select! {
         _=tick.tick()=>{
          if record(root,id)?.status!="running"{
            if let Ok(bytes)=std::fs::read(directory(root,id)?.join("tail.json")) {
                let tail:Value=serde_json::from_slice(&bytes)?;
                let bytes:Vec<u8>=serde_json::from_value(tail["bytes"].clone())?;
                let start=tail["start"].as_u64().unwrap_or(0);
                if offset<start {std::io::stdout().write_all(b"\r\n[Earlier terminal output omitted]\r\n")?;}
                std::io::stdout().write_all(&bytes[offset.saturating_sub(start).min(bytes.len() as u64) as usize..])?;
                std::io::stdout().flush()?;
            }
            break;
          }
          let reply=request(root,id,json!({"action":"output","offset":offset})).await?;
          let bytes:Vec<u8>=serde_json::from_value(reply["bytes"].clone())?;offset=reply["offset"].as_u64().context("Output cursor missing")?;
          std::io::stdout().write_all(&bytes)?;std::io::stdout().flush()?;
         },
         e=events.next()=>{match e.transpose()? {
          Some(Event::Key(k))=>{if k.modifiers.contains(KeyModifiers::CONTROL)&&matches!(k.code,KeyCode::Char(']')|KeyCode::Char('5')){break;}
                    if let Some(text)=key_text(k){request(root,id,json!({"action":"input","text":text})).await?;}},
          Some(Event::Paste(text))=>{request(root,id,json!({"action":"input","text":text})).await?;},
          Some(Event::Resize(cols,rows))=>{request(root,id,json!({"action":"resize","rows":rows,"cols":cols})).await?;},
          None=>break,_=>{}
         }}
        }
    }
    Ok(())
}
pub fn key_text(k: crossterm::event::KeyEvent) -> Option<String> {
    use crossterm::event::{KeyCode, KeyModifiers};
    let s = match k.code {
        KeyCode::Char(c @ '4'..='7') if k.modifiers.contains(KeyModifiers::CONTROL) => {
            ((c as u8 - b'4' + 0x1c) as char).to_string()
        }
        KeyCode::Char(c) if k.modifiers.contains(KeyModifiers::CONTROL) && c.is_ascii() => {
            (((c.to_ascii_lowercase() as u8) & 0x1f) as char).to_string()
        }
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "\r".into(),
        KeyCode::Backspace => "\x7f".into(),
        KeyCode::Tab => "\t".into(),
        KeyCode::Esc => "\x1b".into(),
        KeyCode::Up => "\x1b[A".into(),
        KeyCode::Down => "\x1b[B".into(),
        KeyCode::Right => "\x1b[C".into(),
        KeyCode::Left => "\x1b[D".into(),
        KeyCode::Home => "\x1b[H".into(),
        KeyCode::End => "\x1b[F".into(),
        KeyCode::Delete => "\x1b[3~".into(),
        KeyCode::PageUp => "\x1b[5~".into(),
        KeyCode::PageDown => "\x1b[6~".into(),
        KeyCode::BackTab => "\x1b[Z".into(),
        KeyCode::F(n @ 1..=4) => format!("\x1bO{}", (b'P' + n - 1) as char),
        KeyCode::F(n @ 5..=12) => {
            format!("\x1b[{}~", [15, 17, 18, 19, 20, 21, 23, 24][n as usize - 5])
        }
        _ => return None,
    };
    Some(if k.modifiers.contains(KeyModifiers::ALT) {
        format!("\x1b{s}")
    } else {
        s
    })
}
