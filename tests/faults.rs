#[path = "fixtures/upstream.rs"]
mod upstream;

use alt_cli::project::{Policy, Project};
use std::{fs, process::Command};
#[test]
fn fault_worker() {
    let Ok(data) = std::env::var("ALT_FAULT_DATA") else {
        return;
    };
    let cwd = std::env::var("ALT_FAULT_PROJECT").unwrap();
    let id = std::env::var("ALT_FAULT_CHANGE").unwrap();
    let p = Project::open(std::path::Path::new(&data), std::path::Path::new(&cwd)).unwrap();
    let result = if std::env::var("ALT_FAULT_OPERATION").unwrap() == "apply" {
        p.apply(&id, Policy::Guided)
    } else {
        p.undo(&id)
    };
    if std::env::var("ALT_FAULT_MODE").unwrap() == "enospc" {
        let error = result.unwrap_err();
        assert!(error.to_string().contains("No space left"), "{error:#}");
    } else {
        panic!("Crash injector did not activate");
    }
}
#[test]
fn disk_full_and_abrupt_apply_undo_recover_without_replay_or_partial_source() {
    let tooling = tempfile::tempdir().unwrap();
    let library = tooling.path().join("fault.so");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2"])
        .arg(format!(
            "{}/tests/fixtures/fault_io.c",
            env!("CARGO_MANIFEST_DIR")
        ))
        .args(["-ldl", "-o"])
        .arg(&library)
        .status()
        .unwrap();
    assert!(status.success());
    for operation in ["apply", "undo"] {
        for mode in ["enospc", "crash-before-rename", "crash-after-rename"] {
            let data = tempfile::tempdir().unwrap();
            let cwd = tempfile::tempdir().unwrap();
            fs::write(cwd.path().join("app.py"), "before\n").unwrap();
            let id;
            {
                let p = Project::open(data.path(), cwd.path()).unwrap();
                p.start_task("t", "Fault recovery").unwrap();
                p.note("t", "plan", "Update source", "user").unwrap();
                p.read("t", "app.py", 1, 20).unwrap();
                let c = p
                    .prepare_edit(
                        "t",
                        "app.py",
                        None,
                        "before",
                        "after",
                        "replace",
                        "Fixture change",
                    )
                    .unwrap();
                id = c.id;
                if operation == "undo" {
                    p.apply(&id, Policy::Guided).unwrap();
                }
            }
            let result = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "fault_worker", "--nocapture"])
                .env("LD_PRELOAD", &library)
                .env("ALT_FAULT_DATA", data.path())
                .env("ALT_FAULT_PROJECT", cwd.path())
                .env("ALT_FAULT_CHANGE", &id)
                .env("ALT_FAULT_OPERATION", operation)
                .env("ALT_FAULT_MODE", mode)
                .output()
                .unwrap();
            assert_eq!(
                result.status.code(),
                Some(match mode {
                    "enospc" => 0,
                    "crash-before-rename" => 86,
                    _ => 87,
                }),
                "{operation} {mode}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            let completed = mode == "crash-after-rename";
            let expected = match (operation, completed) {
                ("apply", true) | ("undo", false) => "after\n",
                _ => "before\n",
            };
            assert_eq!(
                fs::read_to_string(cwd.path().join("app.py")).unwrap(),
                expected
            );
            {
                let p = Project::open(data.path(), cwd.path()).unwrap();
                let status = p.change(&id).unwrap().status;
                assert_eq!(
                    status,
                    match (operation, completed) {
                        ("apply", false) => "proposed",
                        ("undo", true) => "undone",
                        _ => "applied",
                    }
                );
            }
            // A second open reconciles records only; never automatically repeats the action.
            drop(Project::open(data.path(), cwd.path()).unwrap());
            assert_eq!(
                fs::read_to_string(cwd.path().join("app.py")).unwrap(),
                expected
            );
        }
    }
}

// ---- A04: hosted-provider resilience (retries and degenerate output) ----

mod provider {
    use super::upstream::{self, Reply};
    use alt_cli::{
        config::{Profile, Provider},
        inference::{OutputGuard, Relay, Settings, degenerate_output},
    };
    use serde_json::{Value, json};

    fn ok(text: &str) -> Reply {
        Reply::json(
            json!({"choices":[{"message":{"role":"assistant","content":text},"finish_reason":"stop"}],"usage":{"completion_tokens":10}}),
        )
    }

    async fn relay(
        server: &upstream::Upstream,
        configure: impl FnOnce(&mut Settings),
    ) -> (tempfile::TempDir, Relay) {
        let mut settings = Settings::legacy(8192);
        settings.output_tokens = 256;
        settings.action_headroom = 64;
        settings.max_requests = Some(10);
        settings.total_generated_tokens = Some(4096);
        configure(&mut settings);
        let profile = Profile {
            provider: Provider::Openai,
            endpoint: server.endpoint.clone(),
            model: "fixture".into(),
            context_tokens: 8192,
            max_turns: 2,
            uncensored: false,
            api_key_env: None,
            local_model: None,
            inference: Some(settings),
            runtime_key: None,
        };
        let root = tempfile::tempdir().unwrap();
        let relay = Relay::start(root.path(), &profile).await.unwrap();
        (root, relay)
    }

    async fn ask(relay: &Relay) -> (u16, Value) {
        let response = reqwest::Client::new()
            .post(format!("{}/chat/completions", relay.endpoint))
            .json(&json!({"model":"fixture","messages":[{"role":"user","content":"Please fix the parser."}],"stream":false}))
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        (status, response.json().await.unwrap_or(Value::Null))
    }

    fn receipt(relay: &Relay) -> Value {
        let path = std::fs::read_dir(&relay.evidence)
            .unwrap()
            .map(|f| f.unwrap().path())
            .find(|p| p.to_string_lossy().ends_with("-receipt.json"))
            .unwrap();
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    #[tokio::test]
    async fn rate_limits_and_server_errors_are_retried_and_counted() {
        let server = upstream::start(vec![
            Reply::status(429, r#"{"error":"slow down"}"#).header("Retry-After", "0"),
            Reply::status(503, r#"{"error":"busy"}"#).header("Retry-After", "0"),
            ok("fixed"),
        ])
        .await;
        let (_root, relay) = relay(&server, |_| {}).await;
        let (status, body) = ask(&relay).await;
        assert_eq!(status, 200);
        assert_eq!(body["choices"][0]["message"]["content"], "fixed");
        assert_eq!(server.bodies().len(), 3);
        let receipt = receipt(&relay);
        assert_eq!(receipt["retries"], 2);
        assert_eq!(receipt["retry_log"][0]["status"], 429);
        assert_eq!(receipt["retry_log"][1]["status"], 503);
        let live = relay.status();
        assert_eq!(live.provider_retries, 2);
        assert_eq!(
            live.remaining_requests,
            Some(7),
            "each retry is a model call"
        );
    }

    #[tokio::test]
    async fn a_dropped_connection_is_retried_and_exhausted_retries_return_the_error() {
        let server = upstream::start(vec![Reply::hang_up(), ok("after reset")]).await;
        let (_root, relay_ok) = relay(&server, |_| {}).await;
        let (status, body) = ask(&relay_ok).await;
        assert_eq!(status, 200);
        assert_eq!(body["choices"][0]["message"]["content"], "after reset");
        assert_eq!(receipt(&relay_ok)["retries"], 1);

        let failing = upstream::start(vec![
            Reply::status(500, r#"{"error":"down"}"#).header("Retry-After", "0"),
        ])
        .await;
        let (_root, relay_bad) = relay(&failing, |s| s.retries = 1).await;
        let (status, _) = ask(&relay_bad).await;
        assert_eq!(status, 500, "the final provider status reaches the engine");
        assert_eq!(failing.bodies().len(), 2);
        let receipt = receipt(&relay_bad);
        assert_eq!(receipt["retries"], 1);
        assert_eq!(receipt["complete"], false);
    }

    #[tokio::test]
    async fn degenerate_output_is_discarded_once_as_a_provider_fault() {
        let server = upstream::start(vec![
            ok("<|open|>函数 Привет <|close|> garbage"),
            ok("Here is the fix."),
        ])
        .await;
        let (_root, relay) = relay(&server, |s| s.output_guard = OutputGuard::On).await;
        let (status, body) = ask(&relay).await;
        assert_eq!(status, 200);
        assert_eq!(body["choices"][0]["message"]["content"], "Here is the fix.");
        let receipt = receipt(&relay);
        assert_eq!(receipt["provider_faults"].as_array().unwrap().len(), 1);
        assert!(
            receipt["provider_faults"][0]["reason"]
                .as_str()
                .unwrap()
                .contains("<|open|>")
        );
        assert_eq!(
            receipt["charged_generated_tokens"], 20,
            "discarded tokens stay charged"
        );
        assert!(std::fs::read_dir(&relay.evidence).unwrap().any(|f| {
            f.unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with("-discarded-1.raw")
        }));
        assert_eq!(relay.status().provider_faults, 1);
        assert_eq!(relay.status().remaining_requests, Some(8));
    }

    #[tokio::test]
    async fn repeated_degenerate_output_becomes_an_error_never_model_text() {
        let server = upstream::start(vec![ok("<|im_start|>assistant<|im_end|> xx")]).await;
        let (_root, relay) = relay(&server, |s| s.output_guard = OutputGuard::On).await;
        let (status, body) = ask(&relay).await;
        assert_eq!(status, 502);
        assert!(
            body["error"]["message"]
                .as_str()
                .unwrap()
                .contains("provider fault")
        );
        assert_eq!(server.bodies().len(), 2, "exactly one resend");
        assert_eq!(receipt(&relay)["status"], "provider_fault");
    }

    #[test]
    fn the_detector_respects_the_conversation_language() {
        let english =
            json!({"messages":[{"role":"user","content":"Please fix the parser in main.rs"}]});
        let russian =
            json!({"messages":[{"role":"user","content":"Пожалуйста, исправь парсер в main.rs"}]});
        let reply = |text: &str| {
            json!({"choices":[{"message":{"content":text},"finish_reason":"stop"}]}).to_string()
        };
        let cyrillic = reply(
            "Я исправил парсер: теперь он корректно обрабатывает пустые строки и комментарии.",
        );
        assert!(degenerate_output(cyrillic.as_bytes(), false, &russian).is_none());
        // A whole answer in one other language is a model choice, not a fault.
        assert!(degenerate_output(cyrillic.as_bytes(), false, &english).is_none());
        let garbage = reply(
            "the parser 修复了 он исправлен 函数调用 строки 解析器 комментарии 处理 файл 结束 конец",
        );
        assert!(degenerate_output(garbage.as_bytes(), false, &english).is_some());
        let normal =
            reply("I fixed the parser so empty lines and comments are skipped correctly now.");
        assert!(degenerate_output(normal.as_bytes(), false, &english).is_none());
        let code = reply("Use `if line.trim().is_empty() { continue; }` — done. ✓ λ");
        assert!(degenerate_output(code.as_bytes(), false, &english).is_none());
    }

    #[test]
    fn the_guard_is_automatic_only_for_remote_servers() {
        let mut settings = Settings::legacy(8192);
        let profile = |endpoint: &str| Profile {
            provider: Provider::Openai,
            endpoint: endpoint.into(),
            model: "m".into(),
            context_tokens: 8192,
            max_turns: 2,
            uncensored: false,
            api_key_env: None,
            local_model: None,
            inference: None,
            runtime_key: None,
        };
        assert!(!settings.guard_active(&profile("http://127.0.0.1:8080/v1")));
        assert!(!settings.guard_active(&profile("http://localhost:11434")));
        assert!(settings.guard_active(&profile("https://integrate.api.nvidia.com/v1")));
        settings.output_guard = OutputGuard::Off;
        assert!(!settings.guard_active(&profile("https://integrate.api.nvidia.com/v1")));
    }
}
