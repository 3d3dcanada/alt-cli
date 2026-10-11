//! Scripted OpenAI-compatible upstream for relay tests. No model weights.
//! Each accepted connection receives the next scripted reply; every request
//! head and JSON body is recorded for assertions.
#![allow(dead_code)]
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone)]
pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub content_type: &'static str,
    pub body: String,
}
impl Reply {
    pub fn json(body: serde_json::Value) -> Self {
        Self {
            status: 200,
            headers: vec![],
            content_type: "application/json",
            body: body.to_string(),
        }
    }
    pub fn sse(events: &[serde_json::Value]) -> Self {
        let mut body: String = events.iter().map(|e| format!("data: {e}\n\n")).collect();
        body.push_str("data: [DONE]\n\n");
        Self {
            status: 200,
            headers: vec![],
            content_type: "text/event-stream",
            body,
        }
    }
    pub fn status(status: u16, body: &str) -> Self {
        Self {
            status,
            headers: vec![],
            content_type: "application/json",
            body: body.into(),
        }
    }
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

pub struct Upstream {
    pub endpoint: String,
    pub requests: Arc<Mutex<Vec<(String, serde_json::Value)>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Upstream {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Upstream {
    pub fn bodies(&self) -> Vec<serde_json::Value> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(|r| r.1.clone())
            .collect()
    }
}

/// Serve `replies` in order (one per connection); the last reply repeats.
pub async fn start(replies: Vec<Reply>) -> Upstream {
    let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/v1", socket.local_addr().unwrap());
    let requests: Arc<Mutex<Vec<(String, serde_json::Value)>>> = Arc::default();
    let seen = requests.clone();
    let task = tokio::spawn(async move {
        let mut index = 0usize;
        loop {
            let Ok((mut connection, _)) = socket.accept().await else {
                break;
            };
            let reply = replies[index.min(replies.len() - 1)].clone();
            index += 1;
            let mut data = Vec::new();
            let (end, length) = loop {
                let mut chunk = [0u8; 8192];
                let n = connection.read(&mut chunk).await.unwrap_or(0);
                if n == 0 {
                    break (0, 0);
                }
                data.extend_from_slice(&chunk[..n]);
                if let Some(end) = data.windows(4).position(|b| b == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&data[..end]).to_string();
                    let length = head
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if data.len() >= end + 4 + length {
                        break (end, length);
                    }
                }
            };
            if end == 0 {
                continue;
            }
            let head = String::from_utf8_lossy(&data[..end]).to_string();
            let body = serde_json::from_slice(&data[end + 4..end + 4 + length])
                .unwrap_or(serde_json::Value::Null);
            seen.lock().unwrap().push((head, body));
            let extra: String = reply
                .headers
                .iter()
                .map(|(k, v)| format!("{k}: {v}\r\n"))
                .collect();
            let _ = connection
                .write_all(
                    format!(
                        "HTTP/1.1 {} Fixture\r\nContent-Type: {}\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n{}",
                        reply.status,
                        reply.content_type,
                        reply.body.len(),
                        reply.body
                    )
                    .as_bytes(),
                )
                .await;
        }
    });
    Upstream {
        endpoint,
        requests,
        task,
    }
}
