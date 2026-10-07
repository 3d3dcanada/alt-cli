use alt_cli::{
    config::{Profile, Provider},
    inference::Settings,
    project::{Policy, Project},
};
use serde_json::json;

fn profile() -> Profile {
    Profile {
        provider: Provider::Openai,
        endpoint: "http://127.0.0.1:8080/v1".into(),
        model: "selected".into(),
        context_tokens: 8192,
        max_turns: 12,
        uncensored: true,
        api_key_env: None,
        local_model: None,
        inference: None,
    }
}
#[test]
fn output_settings_override_engine_defaults_and_reject_fake_support() {
    let p = profile();
    let mut settings = Settings::legacy(8192);
    settings.output_tokens = 3072;
    settings.temperature = Some(0.7);
    settings.top_p = Some(0.8);
    let mut body =
        json!({"model":"selected","max_tokens":99,"temperature":0,"top_k":40,"stream":true});
    settings.apply(&p, &mut body).unwrap();
    assert_eq!(body["max_tokens"], 3072);
    assert_eq!(body["temperature"], 0.7);
    assert!(body.get("top_k").is_none());
    assert_eq!(body["stream_options"]["include_usage"], true);
    body["model"] = json!("hidden-fallback");
    assert!(settings.apply(&p, &mut body).is_err());
    settings.reasoning_tokens = Some(3000);
    assert!(settings.validate(&p).is_err());
    settings.reasoning_tokens = Some(512);
    assert!(settings.validate(&p).is_err());
    settings.llama_extensions = true;
    settings.validate(&p).unwrap();
}
#[test]
fn handles_preserve_unicode_crlf_and_duplicate_symbols_and_undo() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let original = "# ☃\r\ndef same():\r\n return 1\r\ndef same():\r\n return 2\r\n";
    std::fs::write(cwd.path().join("a.py"), original).unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    p.start_task("task", "repair").unwrap();
    p.note("task", "plan", "Inspect patch check", "user")
        .unwrap();
    let read = p.read("task", "a.py", 1, 20).unwrap();
    let symbols = read["symbols"].as_array().unwrap();
    assert_eq!(symbols.len(), 2);
    assert_ne!(symbols[0]["handle"], symbols[1]["handle"]);
    let handle = symbols[1]["handle"].as_str().unwrap();
    let change = p
        .prepare_handle_edit(
            "task",
            "a.py",
            handle,
            "def same():\r\n return 3",
            "Fix second function",
        )
        .unwrap();
    p.apply(&change.id, Policy::Trusted).unwrap();
    let after = std::fs::read_to_string(cwd.path().join("a.py")).unwrap();
    assert!(after.contains("return 1\r\n"));
    assert!(after.contains("return 3\r\n"));
    assert!(after.starts_with("# ☃\r\n"));
    assert!(
        p.prepare_handle_edit("task", "a.py", handle, "bad", "stale")
            .is_err()
    );
    p.undo(&change.id).unwrap();
    assert_eq!(
        std::fs::read_to_string(cwd.path().join("a.py")).unwrap(),
        original
    );
    assert!(
        p.prepare_handle_edit("other", "a.py", handle, "bad", "mismatch")
            .is_err()
    );
}
#[test]
fn concurrent_change_between_prepare_and_apply_never_overwrites_user() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    std::fs::write(cwd.path().join("a.txt"), "a\na\n").unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    p.start_task("task", "repair").unwrap();
    p.note("task", "plan", "Inspect patch check", "user")
        .unwrap();
    let read = p.read("task", "a.txt", 2, 1).unwrap();
    let change = p
        .prepare_handle_edit(
            "task",
            "a.txt",
            read["range_handle"].as_str().unwrap(),
            "b\n",
            "Second repeated line",
        )
        .unwrap();
    std::fs::write(cwd.path().join("a.txt"), "user edit\n").unwrap();
    assert!(p.apply(&change.id, Policy::Trusted).is_err());
    assert_eq!(
        std::fs::read_to_string(cwd.path().join("a.txt")).unwrap(),
        "user edit\n"
    );
}
#[test]
fn host_workflow_and_hypotheses_survive_restart_without_becoming_evidence() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    std::fs::write(
        cwd.path().join("a.py"),
        "from vendor.math import total\ndef calc(x): return total(x)\n",
    )
    .unwrap();
    std::fs::create_dir(cwd.path().join("vendor")).unwrap();
    std::fs::write(
        cwd.path().join("vendor/math.py"),
        "def total(x): return sum(x)\n",
    )
    .unwrap();
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        p.start_task("t", "repair calc total").unwrap();
        alt_cli::workflow::begin(&p, "t", alt_cli::workflow::Mode::Host).unwrap();
        p.note(
            "t",
            "hypothesis",
            "Maybe the caller is wrong",
            "model note (unverified)",
        )
        .unwrap();
    }
    let mut p = Project::open(data.path(), cwd.path()).unwrap();
    let packet = alt_cli::workflow::packet(&p, "t").unwrap();
    assert_eq!(packet["hypotheses"][0]["verified"], false);
    assert_eq!(packet["stage"], "inspect");
    assert!(
        !packet["facts"]["verification"]["behavioral_acceptance"]
            .as_bool()
            .unwrap()
    );
    let outline = p.structural_context("calc").unwrap();
    assert!(outline.iter().any(|v| {
        v["neighbors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["path"] == "vendor/math.py")
    }));
    let before = outline[0]["sha256"].clone();
    std::fs::write(cwd.path().join("a.py"), "def calc(x): return x\n").unwrap();
    let after = p.structural_context("calc").unwrap();
    assert_ne!(after[0]["sha256"], before);
    assert!(after[0]["dependencies"].as_array().unwrap().is_empty());
}
#[test]
fn skills_are_bounded_and_do_not_grant_permissions() {
    let skills = alt_cli::skills::catalog();
    assert_eq!(skills.len(), 6);
    for skill in skills {
        assert_eq!(skill.license, "Apache-2.0");
        assert_eq!(skill.sha256.len(), 64);
        assert!(skill.procedure.len() <= 5000);
        assert!(!skill.helpers.is_empty());
    }
    assert_eq!(
        alt_cli::skills::shortlist("Rust cargo compiler diagnostics")[0].id,
        "rust-diagnostics"
    );
    assert!(alt_cli::skills::get("unknown").is_err());
}
#[test]
fn native_decoder_rejects_partial_calls_and_preserves_unicode_fragments() {
    let raw = concat!(
        "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"c\",\"function\":{\"name\":\"echo\",\"arguments\":\"{\\\"text\\\":\"}}]},\"finish_reason\":null}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"\\\"☃\\\"}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n\n",
        "data: [DONE]\n\n"
    );
    let decoded = alt_cli::native::decode(raw.as_bytes(), true).unwrap();
    assert_eq!(
        decoded["tool_calls"][0]["function"]["arguments"],
        "{\"text\":\"☃\"}"
    );
    assert!(alt_cli::native::decode(raw.replace("data: [DONE]\n\n", "").as_bytes(), true).is_err());
    for reason in ["length", "content_filter", "cancelled", "unknown"] {
        assert!(
            alt_cli::native::decode(
                raw.replace(
                    "\"finish_reason\":\"tool_calls\"",
                    &format!("\"finish_reason\":\"{reason}\""),
                )
                .as_bytes(),
                true
            )
            .is_err(),
            "An incomplete decision cannot pass: {reason}"
        );
    }
    assert!(alt_cli::native::decode(b"data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":-1}]}}]}\n\ndata: [DONE]\n\n",true).is_err());
}
#[test]
fn instruction_drafts_need_independent_validation_and_can_rollback() {
    let data = tempfile::tempdir().unwrap();
    let meta = json!({"schema":1,"target":"operator-supplement","optimizer":{"kind":"human"},"development_families":["python-feature"],"validation_families":["python-config"],"review":{"status":"pending"},"validation_evidence":[],"cost":{"requests":1,"wall_seconds":1.0}});
    let registered = alt_cli::instructions::register(
        data.path(),
        "Read current handles; use actual check evidence.\n",
        &meta,
    )
    .unwrap();
    let id = registered["id"].as_str().unwrap();
    let campaign = data.path().join("validation");
    let attempt = campaign.join("python-config");
    std::fs::create_dir_all(attempt.join("project")).unwrap();
    std::fs::write(
        campaign.join("configuration.json"),
        serde_json::to_vec(
            &json!({"oracle_version":5,"partition":"development","instruction_sha256":id}),
        )
        .unwrap(),
    )
    .unwrap();
    std::fs::write(attempt.join("project/solution.py"), "fixed\n").unwrap();
    let row = json!({"case":"python-config","passed":true,"oracle_unchanged":true,"before":{"passed":false},"after":{"passed":true,"exit":0,"completion_observed":true},"resulting_source":{"solution.py":"fixed\n"}});
    let evidence = attempt.join("report.json");
    std::fs::write(&evidence, serde_json::to_vec(&row).unwrap()).unwrap();
    let request = attempt.join("probe-request.json");
    std::fs::write(&request,serde_json::to_vec(&json!({"messages":[{"role":"user","content":format!("Experimental operator supplement {id}")}]})).unwrap()).unwrap();
    std::fs::write(attempt.join("evidence-sha256.json"),serde_json::to_vec(&json!({"report.json":alt_cli::capability::file_sha256(&evidence).unwrap(),"probe-request.json":alt_cli::capability::file_sha256(&request).unwrap()})).unwrap()).unwrap();
    let mut prefs = alt_cli::config::Preferences::default();
    assert!(alt_cli::instructions::activate(data.path(), &mut prefs, Some(id)).is_err());
    let receipt = json!({"instruction_sha256":id,"review":{"status":"approved","reviewer":"fixture"},"validation_evidence":[{"path":evidence,"sha256":alt_cli::capability::file_sha256(&evidence).unwrap()}]});
    alt_cli::instructions::review(data.path(), id, &receipt).unwrap();
    alt_cli::instructions::activate(data.path(), &mut prefs, Some(id)).unwrap();
    assert_eq!(prefs.instruction_version.as_deref(), Some(id));
    alt_cli::instructions::activate(data.path(), &mut prefs, None).unwrap();
    assert!(prefs.instruction_version.is_none());
    std::fs::write(evidence, "changed").unwrap();
    assert!(alt_cli::instructions::activate(data.path(), &mut prefs, Some(id)).is_err());
    let mut leaked = meta;
    leaked["development_families"] = json!(["sealed-intervals"]);
    assert!(alt_cli::instructions::register(data.path(), "bad", &leaked).is_err());
    let metadata = data
        .path()
        .join("instructions")
        .join(id)
        .join("metadata.json");
    std::fs::write(metadata, "{}").unwrap();
    assert!(alt_cli::instructions::metadata(data.path(), id).is_err());
}
#[test]
fn analysis_source_chunks_remain_bounded_at_unicode_boundaries() {
    let chunks = alt_cli::analysis::chunks("é☃\r\nalpha", 5, 2);
    assert_eq!(chunks[0], (0, 5, "é☃".into()));
    assert_eq!(chunks.len(), 2);
    assert!(
        chunks
            .iter()
            .all(|(a, b, s)| b - a == s.len() && s.len() <= 5)
    );
    let selected = alt_cli::analysis::relevant_chunks(
        "alpha xxxxxxxxxxxxx cedar target",
        8,
        1,
        "cedar target",
    );
    assert!(selected[0].0 > 0);
    assert!(selected[0].2.contains("cedar") || selected[0].2.contains("target"));
}

#[test]
fn partial_stream_is_not_complete_cost_evidence() {
    let complete=b"data: {\"choices\":[{\"finish_reason\":\"stop\",\"delta\":{}}],\"usage\":{\"completion_tokens\":1}}\n\ndata: [DONE]\n\n";
    assert!(alt_cli::inference::response_complete(complete, true));
    assert!(!alt_cli::inference::response_complete(
        b"data: {\"usage\":{\"completion_tokens\":1}}\n\n",
        true
    ));
    assert!(!alt_cli::inference::response_complete(
        b"data: [DONE]\n\ndata: {}\n\n",
        true
    ));
    let mut p = profile();
    let mut settings = Settings::legacy(p.context_tokens);
    settings.output_parameter = alt_cli::inference::OutputParameter::MaxCompletionTokens;
    p.inference = Some(settings.clone());
    let mut body = json!({"model":p.model,"max_tokens":1});
    settings.apply(&p, &mut body).unwrap();
    assert!(body.get("max_tokens").is_none());
    assert_eq!(body["max_completion_tokens"], 2048);
}
