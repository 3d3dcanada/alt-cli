use alt_cli::{
    config::{Profile, Provider},
    inference::{Relay, Settings},
};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn incomplete_stream_keeps_its_reserved_cost_and_cannot_buy_another_call() {
    let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = socket.local_addr().unwrap().port();
    let upstream = tokio::spawn(async move {
        let (mut connection, _) = socket.accept().await.unwrap();
        let mut data = Vec::new();
        loop {
            let mut chunk = [0u8; 4096];
            let n = connection.read(&mut chunk).await.unwrap();
            assert!(n > 0);
            data.extend_from_slice(&chunk[..n]);
            if let Some(end) = data.windows(4).position(|b| b == b"\r\n\r\n") {
                let head = String::from_utf8_lossy(&data[..end]);
                let length = head
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|n| n.trim().parse::<usize>().unwrap())
                    })
                    .unwrap();
                if data.len() >= end + 4 + length {
                    break;
                }
            }
        }
        // A usage claim without a finished SSE exchange must not refund the reservation.
        let body = "data: {\"usage\":{\"completion_tokens\":0},\"choices\":[]}\n\n";
        connection
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });
    let mut settings = Settings::legacy(8192);
    settings.output_tokens = 128;
    settings.action_headroom = 64;
    settings.total_generated_tokens = Some(128);
    settings.max_requests = Some(2);
    let profile = Profile {
        provider: Provider::Openai,
        endpoint: format!("http://127.0.0.1:{port}/v1"),
        model: "fixture".into(),
        context_tokens: 8192,
        max_turns: 2,
        uncensored: true,
        api_key_env: None,
        local_model: None,
        inference: Some(settings),
    };
    let root = tempfile::tempdir().unwrap();
    let relay = Relay::start(root.path(), &profile).await.unwrap();
    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", relay.endpoint);
    let body =
        json!({"model":"fixture","messages":[{"role":"user","content":"fixture"}],"stream":true});
    let first = client.post(&url).json(&body).send().await.unwrap();
    assert!(first.status().is_success());
    first.bytes().await.unwrap();
    let second = client.post(&url).json(&body).send().await.unwrap();
    assert_eq!(second.status().as_u16(), 429);
    let cost = alt_cli::inference::cost(&relay.evidence).unwrap();
    assert_eq!(cost["requests"], 1);
    assert_eq!(cost["charged_generated_tokens"], 128);
    assert!(cost["known_generated_tokens"].is_null());
    let receipt = std::fs::read_dir(&relay.evidence)
        .unwrap()
        .map(|f| f.unwrap().path())
        .find(|p| p.to_string_lossy().ends_with("-receipt.json"))
        .unwrap();
    let receipt: serde_json::Value =
        serde_json::from_slice(&std::fs::read(receipt).unwrap()).unwrap();
    assert_eq!(receipt["complete"], false);
    upstream.await.unwrap();
}

#[tokio::test]
async fn full_chat_and_tools_are_measured_before_spending_or_forwarding() {
    let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = socket.local_addr().unwrap().port();
    let upstream = tokio::spawn(async move {
        for step in 0..5 {
            let (mut connection, _) = socket.accept().await.unwrap();
            let mut data = Vec::new();
            let (end, length) = loop {
                let mut chunk = [0u8; 4096];
                let n = connection.read(&mut chunk).await.unwrap();
                assert!(n > 0);
                data.extend_from_slice(&chunk[..n]);
                if let Some(end) = data.windows(4).position(|b| b == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&data[..end]);
                    let length = head
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    if data.len() >= end + 4 + length {
                        break (end, length);
                    }
                }
            };
            let head = String::from_utf8_lossy(&data[..end]);
            let body: serde_json::Value =
                serde_json::from_slice(&data[end + 4..end + 4 + length]).unwrap();
            let response = match step {
                0 | 2 => {
                    assert!(head.starts_with("POST /apply-template "));
                    assert_eq!(body["tools"][0]["function"]["name"], "fixture_tool");
                    assert_eq!(body["max_tokens"], 96);
                    assert_eq!(body["reasoning_budget_tokens"], 32);
                    json!({"prompt":"Rendered system, user, tools and generation prefix ☃"})
                }
                1 | 3 => {
                    assert!(head.starts_with("POST /tokenize "));
                    assert_eq!(body["parse_special"], true);
                    assert!(body["content"].as_str().unwrap().contains("generation prefix"));
                    json!({"tokens":vec![1; if step==1 {2000} else {10}]})
                }
                _ => {
                    assert!(head.starts_with("POST /v1/chat/completions "));
                    assert_eq!(body["max_tokens"], 96);
                    assert_eq!(body["reasoning_budget_tokens"], 32);
                    json!({"choices":[{"message":{"role":"assistant","content":"fixture"},"finish_reason":"stop"}],"usage":{"completion_tokens":20}})
                }
            }.to_string();
            connection.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",response.len()).as_bytes()).await.unwrap();
        }
    });
    let mut settings = Settings::legacy(2048);
    settings.output_tokens = 128;
    settings.action_headroom = 64;
    settings.reasoning_tokens = Some(64);
    settings.total_generated_tokens = Some(96);
    settings.max_requests = Some(1);
    let profile = Profile {
        provider: Provider::Openai,
        endpoint: format!("http://127.0.0.1:{port}/v1"),
        model: "fixture".into(),
        context_tokens: 2048,
        max_turns: 2,
        uncensored: true,
        api_key_env: None,
        local_model: Some("deterministic-protocol-fixture".into()),
        inference: Some(settings),
    };
    let root = tempfile::tempdir().unwrap();
    let relay = Relay::start(root.path(), &profile).await.unwrap();
    let url = format!("{}/chat/completions", relay.endpoint);
    let client = reqwest::Client::new();
    let body = json!({"model":"fixture","max_tokens":99,"messages":[{"role":"system","content":"system"},{"role":"user","content":"user"}],"tools":[{"type":"function","function":{"name":"fixture_tool","parameters":{"type":"object"}}}],"stream":false});
    let rejected = client.post(&url).json(&body).send().await.unwrap();
    assert_eq!(rejected.status().as_u16(), 400);
    rejected.bytes().await.unwrap();
    assert_eq!(
        alt_cli::inference::cost(&relay.evidence).unwrap()["requests"],
        0
    );
    let accepted = client.post(&url).json(&body).send().await.unwrap();
    assert!(accepted.status().is_success());
    accepted.bytes().await.unwrap();
    let cost = alt_cli::inference::cost(&relay.evidence).unwrap();
    assert_eq!(cost["requests"], 1);
    assert_eq!(cost["charged_generated_tokens"], 20);
    upstream.await.unwrap();
}
