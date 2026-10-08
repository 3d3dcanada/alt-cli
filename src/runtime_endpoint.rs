//! A private Unix socket for llama.cpp and a per-launch capability URL for clients.
//! Readiness never trusts a separately bound TCP listener. The proxy retains its
//! listener throughout startup, so another process cannot take its address.
use anyhow::{Context, Result, ensure};
use std::{path::PathBuf, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream, UnixStream},
    task::{JoinHandle, JoinSet},
};

pub(super) struct Endpoint {
    pub url: String,
    pub socket: PathBuf,
    _directory: tempfile::TempDir,
    task: JoinHandle<()>,
}
impl Endpoint {
    pub async fn reserve() -> Result<Self> {
        let directory = tempfile::Builder::new().prefix("alt-runtime-").tempdir()?;
        crate::config::private_dir(directory.path())?;
        let socket = directory.path().join("inference.sock");
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let secret = uuid::Uuid::new_v4().to_string();
        let url = format!("http://{}/{secret}/v1", listener.local_addr()?);
        let target = socket.clone();
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                tokio::select! {
                    accepted=listener.accept()=>match accepted {
                        Ok((stream,_))=>{
                            if connections.len()>=16 { drop(stream); continue; }
                            let target=target.clone();let secret=secret.clone();
                            connections.spawn(async move { let _=forward(stream,target,&secret).await; });
                        },
                        Err(_)=>break,
                    },
                    _=connections.join_next(),if !connections.is_empty()=>{},
                }
            }
        });
        Ok(Self {
            url,
            socket,
            _directory: directory,
            task,
        })
    }
    pub fn client(&self) -> Result<reqwest::Client> {
        Ok(reqwest::Client::builder()
            .no_proxy()
            .unix_socket(self.socket.as_path())
            .timeout(Duration::from_secs(1))
            .build()?)
    }
}
impl Drop for Endpoint {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn forward(stream: TcpStream, target: PathBuf, secret: &str) -> Result<()> {
    let (input, mut output) = stream.into_split();
    let mut input = BufReader::new(input);
    let header = tokio::time::timeout(Duration::from_secs(5), async {
        let mut bytes = Vec::new();
        loop {
            let buffer = input.fill_buf().await?;
            ensure!(!buffer.is_empty(), "Incomplete private runtime request");
            let n = buffer
                .iter()
                .position(|b| *b == b'\n')
                .map(|i| i + 1)
                .unwrap_or(buffer.len());
            ensure!(
                bytes.len() + n <= 64 * 1024,
                "Private runtime headers exceed 64 KiB"
            );
            bytes.extend_from_slice(&buffer[..n]);
            input.consume(n);
            if bytes.ends_with(b"\r\n\r\n") {
                break;
            }
        }
        Ok::<_, anyhow::Error>(bytes)
    })
    .await??;
    let header = std::str::from_utf8(&header)?;
    let mut lines = header.split("\r\n");
    let first = lines.next().context("Private runtime request line")?;
    let mut fields = first.split_whitespace();
    let method = fields.next().context("Request method")?;
    let path = fields.next().context("Request path")?;
    let prefix = format!("/{secret}/");
    let Some(path) = path.strip_prefix(&prefix) else {
        output
            .write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await?;
        return Ok(());
    };
    ensure!(
        matches!(method, "GET" | "POST")
            && fields.next() == Some("HTTP/1.1")
            && fields.next().is_none(),
        "Unsupported private runtime request"
    );
    let mut upstream = UnixStream::connect(target).await?;
    let mut forwarded = format!("{method} /{path} HTTP/1.1\r\n");
    for line in lines.filter(|l| !l.is_empty()) {
        let (name, _) = line.split_once(':').context("Malformed runtime header")?;
        if !name.eq_ignore_ascii_case("connection") {
            forwarded.push_str(line);
            forwarded.push_str("\r\n");
        }
    }
    forwarded.push_str("Connection: close\r\n\r\n");
    upstream.write_all(forwarded.as_bytes()).await?;
    let (mut upstream_read, mut upstream_write) = upstream.split();
    // HTTP request bytes retained by BufReader must be forwarded too. Terminating
    // either side cancels the other copy; a dropped client cannot pin inference.
    tokio::select! {
        result=tokio::io::copy(&mut input,&mut upstream_write)=>{result?;},
        result=tokio::io::copy(&mut upstream_read,&mut output)=>{result?;},
    }
    Ok(())
}
