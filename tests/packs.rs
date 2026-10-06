use alt_cli::{packs, project::Policy};
use serde_json::json;
use std::{
    fs,
    process::Command,
    sync::{Arc, atomic::AtomicBool},
};
#[test]
fn parsers_reject_incomplete_and_malformed_scans_and_preserve_evidence() {
    for pack in ["semgrep", "pip-audit", "npm-audit", "browser"] {
        assert!(packs::parse(pack, "not JSON", "e", &json!({})).is_err());
        assert!(packs::parse(pack, "{}", "e", &json!({})).is_err());
    }
    for (pack, empty) in [
        ("pip-audit", r#"{"dependencies":[]}"#),
        (
            "npm-audit",
            r#"{"vulnerabilities":{},"metadata":{"dependencies":{"total":0}}}"#,
        ),
    ] {
        assert!(packs::parse(pack, empty, "e", &json!({})).is_err());
    }
    assert!(
        packs::parse(
            "pip-audit",
            r#"{"dependencies":[{"name":"example","version":"1","vulns":[]}]}"#,
            "e",
            &json!({})
        )
        .unwrap()
        .is_empty()
    );
    assert!(
        packs::parse(
            "npm-audit",
            r#"{"vulnerabilities":{},"metadata":{"dependencies":{"total":1}}}"#,
            "e",
            &json!({})
        )
        .unwrap()
        .is_empty()
    );
    let semgrep = json!({"results":[{"check_id":"rule","path":"app.py","start":{"line":3},"extra":{"message":"unsafe eval","severity":"WARNING"}}],"errors":[],"paths":{"scanned":["app.py"]}});
    let f = packs::parse("semgrep", &semgrep.to_string(), "record-id", &json!({})).unwrap();
    assert_eq!(f[0].evidence, "record-id");
    assert_eq!(f[0].line, Some(3));
    assert_eq!(f[0].severity, "medium");
    assert!(
        packs::parse(
            "semgrep",
            r#"{"results":[],"errors":[{"message":"scan failed"}]}"#,
            "e",
            &json!({})
        )
        .is_err()
    );
    assert!(
        packs::parse(
            "pip-audit",
            r#"{"dependencies":[{"name":"missing","skip_reason":"unavailable"}]}"#,
            "e",
            &json!({})
        )
        .is_err()
    );
    assert_eq!(
        packs::parse(
            "browser",
            r#"{"passed":false,"error":"button missing"}"#,
            "e",
            &json!({})
        )
        .unwrap()
        .len(),
        1
    );
}
fn git(cwd: &std::path::Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout).unwrap()
}
#[tokio::test]
async fn git_commit_contains_selected_paths_and_preserves_unrelated_staging() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    git(cwd.path(), &["init", "-q"]);
    git(cwd.path(), &["config", "user.name", "Fixture"]);
    git(
        cwd.path(),
        &["config", "user.email", "fixture@example.invalid"],
    );
    for p in ["chosen.txt", "unrelated.txt"] {
        fs::write(cwd.path().join(p), "before\n").unwrap();
    }
    git(cwd.path(), &["add", "."]);
    git(cwd.path(), &["commit", "-qm", "initial"]);
    fs::write(cwd.path().join("chosen.txt"), "chosen change\n").unwrap();
    fs::write(cwd.path().join("unrelated.txt"), "unrelated change\n").unwrap();
    git(cwd.path(), &["add", "unrelated.txt"]);
    let result = packs::run(
        data.path(),
        cwd.path(),
        "git",
        json!({"action":"commit","message":"Selected change","paths":["chosen.txt"]}),
        Policy::Trusted,
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .unwrap();
    assert_eq!(result.status, "passed", "{:?}", result.error);
    assert_eq!(
        git(cwd.path(), &["show", "--format=", "--name-only", "HEAD"]).trim(),
        "chosen.txt"
    );
    assert_eq!(
        git(cwd.path(), &["diff", "--cached", "--name-only"]).trim(),
        "unrelated.txt"
    );
    assert!(
        packs::run(
            data.path(),
            cwd.path(),
            "git",
            json!({"action":"stage","paths":[]}),
            Policy::Guided,
            Arc::new(AtomicBool::new(false))
        )
        .await
        .is_err()
    );
}
#[tokio::test]
async fn http_failure_retest_and_exports_link_to_real_records() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        for body in ["broken", "ready"] {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut input = [0u8; 4096];
            assert!(s.read(&mut input).await.unwrap() > 0);
            s.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        }
    });
    let input = json!({"url":url,"contains":"ready"});
    let failed = packs::run(
        data.path(),
        cwd.path(),
        "http",
        input.clone(),
        Policy::Trusted,
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .unwrap();
    assert_eq!(failed.status, "findings");
    let pass = packs::run(
        data.path(),
        cwd.path(),
        "http",
        input.clone(),
        Policy::Trusted,
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .unwrap();
    assert_eq!(pass.status, "passed");
    server.await.unwrap();
    packs::retest(data.path(), cwd.path(), &failed.findings[0].id, &pass.id).unwrap();
    let sarif: serde_json::Value =
        serde_json::from_str(&packs::export(data.path(), cwd.path(), "sarif").unwrap()).unwrap();
    assert_eq!(
        sarif["runs"][0]["results"][0]["properties"]["evidence"],
        failed.id
    );
    assert_eq!(
        sarif["runs"][0]["results"][0]["properties"]["retests"][0]["finding_present"],
        false
    );
    let unavailable = packs::run(
        data.path(),
        cwd.path(),
        "http",
        input,
        Policy::Trusted,
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .unwrap();
    assert_eq!(unavailable.status, "failed");
    assert!(unavailable.error.is_some());
    assert_eq!(packs::runs(data.path(), cwd.path()).unwrap().len(), 3);
}
