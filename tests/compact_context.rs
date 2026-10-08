use alt_cli::{
    project::{Policy, Project, digest},
    toolbox::{self, ToolProfile},
};
use rusqlite::Connection;

fn strings(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|s| s.to_string()).collect()
}

#[test]
fn parser_feedback_reports_missing_definitions_without_forbidding_intentional_changes() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let original = "def scaled(x):\n return x * 2\n";
    std::fs::write(cwd.path().join("repair.py"), original).unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    p.start_task("t", "Change the source").unwrap();
    p.note(
        "t",
        "plan",
        "Review the requested change and check it",
        "user",
    )
    .unwrap();
    let read = p.read("t", "repair.py", 1, 20).unwrap();
    let change = p
        .prepare_text_edit(
            "t",
            "repair.py",
            read["range_handle"].as_str().unwrap(),
            " return x * 3\n",
            "A deliberate fixture regression",
        )
        .unwrap();
    let applied = p.apply(&change.id, Policy::Trusted).unwrap();
    let after = std::fs::read_to_string(cwd.path().join("repair.py")).unwrap();
    let feedback =
        alt_cli::compact_context::syntax_delta("repair.py", applied.before.as_deref(), &after)
            .unwrap();
    assert_eq!(
        feedback["missing_previous_definitions"],
        serde_json::json!(["scaled"])
    );
    assert!(feedback["scope"].as_str().unwrap().contains("intentional"));
    p.undo(&change.id).unwrap();
    assert_eq!(
        std::fs::read_to_string(cwd.path().join("repair.py")).unwrap(),
        original
    );
    let unsupported =
        alt_cli::compact_context::syntax_delta("notes.txt", Some("old"), "new").unwrap();
    assert!(unsupported["parser"].is_null() && unsupported["contains_parse_errors"].is_null());
}

#[test]
fn literal_lines_preserve_crlf_unicode_and_backslashes_and_can_undo() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let original = "# ☃\r\ndef calc():\r\n return 1\r\n# keep\r\n";
    std::fs::write(cwd.path().join("calc.py"), original).unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    p.start_task("t", "repair").unwrap();
    p.note("t", "plan", "Patch then check", "user").unwrap();
    let read = p.read("t", "calc.py", 2, 2).unwrap();
    let handle = read["range_handle"].as_str().unwrap();
    assert!(handle.len() < 24);
    assert_eq!(read["source"], "def calc():\r\n return 1\r\n");
    let c = p
        .prepare_lines_edit(
            "t",
            "calc.py",
            Some(handle),
            &strings(&["def calc():", r" return '\n☃'"]),
            "handle",
            "Literal backslash",
        )
        .unwrap();
    p.apply(&c.id, Policy::Trusted).unwrap();
    assert_eq!(
        std::fs::read_to_string(cwd.path().join("calc.py")).unwrap(),
        "# ☃\r\ndef calc():\r\n return '\\n☃'\r\n# keep\r\n"
    );
    assert!(
        p.prepare_lines_edit(
            "t",
            "calc.py",
            Some(handle),
            &strings(&["wrong"]),
            "handle",
            "Stale"
        )
        .is_err()
    );
    p.undo(&c.id).unwrap();
    assert_eq!(
        std::fs::read_to_string(cwd.path().join("calc.py")).unwrap(),
        original
    );
    assert!(
        p.prepare_lines_edit(
            "other",
            "calc.py",
            Some(handle),
            &strings(&["wrong"]),
            "handle",
            "Other task"
        )
        .is_err()
    );
    std::fs::write(cwd.path().join("copy.py"), original).unwrap();
    p.read("t", "copy.py", 1, 100).unwrap();
    assert!(
        p.prepare_lines_edit(
            "t",
            "copy.py",
            Some(handle),
            &strings(&["wrong"]),
            "handle",
            "Other file"
        )
        .is_err()
    );
}

#[test]
fn line_edits_preserve_missing_final_newline_and_require_real_read_plan_and_valid_lines() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    std::fs::write(cwd.path().join("a.txt"), "same\nsame").unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    p.start_task("t", "repair").unwrap();
    let read = p.read("t", "a.txt", 2, 1).unwrap();
    let handle = read["range_handle"].as_str().unwrap();
    assert!(
        p.prepare_lines_edit(
            "t",
            "a.txt",
            Some(handle),
            &strings(&["b"]),
            "handle",
            "Plan missing"
        )
        .is_err()
    );
    p.note("t", "plan", "Patch", "user").unwrap();
    assert!(
        p.prepare_lines_edit(
            "t",
            "a.txt",
            Some(handle),
            &strings(&["a\nb"]),
            "handle",
            "Malformed line"
        )
        .is_err()
    );
    assert!(
        p.prepare_lines_edit("t", "a.txt", Some(handle), &[], "delete", "Partial delete")
            .is_err()
    );
    let c = p
        .prepare_lines_edit(
            "t",
            "a.txt",
            Some(handle),
            &strings(&["b"]),
            "handle",
            "Second repeated line",
        )
        .unwrap();
    p.apply(&c.id, Policy::Trusted).unwrap();
    assert_eq!(
        std::fs::read_to_string(cwd.path().join("a.txt")).unwrap(),
        "same\nb"
    );
}

#[test]
fn new_and_empty_files_and_explicit_delete_remain_checkpointed() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    p.start_task("t", "new file").unwrap();
    p.note("t", "plan", "Create then check", "user").unwrap();
    let c = p
        .prepare_lines_edit("t", "a.txt", None, &[], "create", "New file")
        .unwrap();
    p.apply(&c.id, Policy::Trusted).unwrap();
    let read = p.read("t", "a.txt", 1, 100).unwrap();
    let c = p
        .prepare_lines_edit(
            "t",
            "a.txt",
            read["range_handle"].as_str(),
            &strings(&["☃"]),
            "handle",
            "Fill empty file",
        )
        .unwrap();
    p.apply(&c.id, Policy::Trusted).unwrap();
    assert_eq!(
        std::fs::read_to_string(cwd.path().join("a.txt")).unwrap(),
        "☃"
    );
    let read = p.read("t", "a.txt", 1, 100).unwrap();
    let c = p
        .prepare_lines_edit(
            "t",
            "a.txt",
            read["range_handle"].as_str(),
            &[],
            "delete",
            "Delete file",
        )
        .unwrap();
    p.apply(&c.id, Policy::Trusted).unwrap();
    assert!(!cwd.path().join("a.txt").exists());
    p.undo(&c.id).unwrap();
    assert_eq!(
        std::fs::read_to_string(cwd.path().join("a.txt")).unwrap(),
        "☃"
    );
}

#[test]
fn migration_retains_legacy_handles_and_short_handles_survive_restart() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    std::fs::write(cwd.path().join("a.txt"), "hello\n").unwrap();
    let db_path;
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        p.start_task("t", "repair").unwrap();
        p.note("t", "plan", "Patch", "user").unwrap();
        p.read("t", "a.txt", 1, 100).unwrap();
        db_path = p.state.join("project.db");
    }
    let db = Connection::open(&db_path).unwrap();
    db.execute_batch("DROP TABLE edit_handles; PRAGMA user_version=3;")
        .unwrap();
    drop(db);
    let handle;
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        let hash = digest(b"hello\n");
        let binding = digest(format!("t\0a.txt\0{hash}\0{}\0{}", 0, 6).as_bytes());
        let legacy = format!("v1:{hash}:0:6:{binding}");
        assert!(
            p.prepare_handle_edit("t", "a.txt", &legacy, "new\n", "Old handle")
                .is_ok()
        );
        handle = p.read("t", "a.txt", 1, 100).unwrap()["range_handle"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(
            std::fs::read_dir(p.state.join("migrations"))
                .unwrap()
                .count()
                > 0
        );
    }
    let p = Project::open(data.path(), cwd.path()).unwrap();
    assert!(
        p.prepare_handle_edit("t", "a.txt", &handle, "new\n", "Persisted handle")
            .is_ok()
    );
    let db = Connection::open(&db_path).unwrap();
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        5
    );
}

#[test]
fn compact_context_contains_current_authorized_source_and_complete_pins_without_duplicate_request()
{
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    std::fs::write(cwd.path().join("a.py"), "def calc(x):\n return x\n").unwrap();
    let mut p = Project::open(data.path(), cwd.path()).unwrap();
    p.start_task("t", "repair a.py calc").unwrap();
    p.note("t", "plan", "Patch then check", "user").unwrap();
    p.pin("Preserve ☃ and public interfaces.").unwrap();
    let memory = p.compact_memory("t", "repair a.py calc", 1800).unwrap();
    assert!(memory.contains("Preserve ☃ and public interfaces."));
    assert!(!memory.contains("repair a.py calc"));
    assert!(memory.contains("def calc(x):\n return x\n"));
    assert!(memory.chars().count() <= 1800);
    let view = p.context_view("t").unwrap();
    let read = &view["current_reads"][0];
    assert!(
        p.prepare_lines_edit(
            "t",
            "a.py",
            read["range_handle"].as_str(),
            &strings(&["def calc(x):", " return x + 1"]),
            "handle",
            "Use actual read"
        )
        .is_ok()
    );
    p.pin(&"a".repeat(1000)).unwrap();
    assert!(p.compact_memory("t", "calc", 800).is_err());
}

#[test]
fn compact_continuation_keeps_complete_original_scope_and_its_relevant_source() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    for name in ["a.py", "b.py", "c.py", "d.py", "target.py"] {
        std::fs::write(cwd.path().join(name), "def answer():\n return 1\n").unwrap();
    }
    let mut p = Project::open(data.path(), cwd.path()).unwrap();
    let goal = format!(
        "Repair target.py. {} Preserve the caller's ☃ contract at the end.",
        "Keep the original public interface. ".repeat(10)
    );
    p.start_task("t", &goal).unwrap();
    p.note("t", "plan", "Patch then check", "user").unwrap();
    assert!(!p.compact_memory("t", &goal, 1800).unwrap().contains(&goal));
    let memory = p.compact_memory("t", "Continue", 1800).unwrap();
    assert_eq!(memory.matches(&goal).count(), 1);
    assert!(memory.contains("latest user message takes precedence"));
    assert!(memory.contains("Current file target.py"));
    assert!(memory.chars().count() <= 1800);
    let read = p.context_view("t").unwrap()["current_reads"][0].clone();
    assert_eq!(read["path"], "target.py");
    assert!(
        p.prepare_text_edit(
            "t",
            "target.py",
            read["range_handle"].as_str().unwrap(),
            "def answer():\n return 2\n",
            "Continue the saved task",
        )
        .is_ok()
    );
    assert!(p.compact_memory("t", "Continue", 300).is_err());
    assert_eq!(p.task("t").unwrap().goal, goal);
}

#[test]
fn clipped_source_never_advertises_an_edit_handle() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    std::fs::write(
        cwd.path().join("a.py"),
        format!("def huge():\n return '{}'\n", "x".repeat(2500)),
    )
    .unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    p.start_task("t", "repair").unwrap();
    let read = p.read("t", "a.py", 1, 100).unwrap();
    assert!(read["range_handle"].is_null());
    assert!(read["symbols"].as_array().unwrap().is_empty());
    assert!(read["source"].is_null());
}

#[test]
fn named_source_prefetch_does_not_index_unrelated_archives() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    std::fs::create_dir(cwd.path().join("archive")).unwrap();
    for i in 0..100 {
        std::fs::write(
            cwd.path().join(format!("archive/{i}.txt")),
            "unrelated recorded tool trace",
        )
        .unwrap();
    }
    std::fs::write(cwd.path().join("calc.py"), "def calc(x): return x\n").unwrap();
    let mut p = Project::open(data.path(), cwd.path()).unwrap();
    p.start_task("t", "repair calc.py").unwrap();
    let memory = p.compact_memory("t", "Repair calc.py.", 1400).unwrap();
    assert!(memory.contains("def calc(x): return x"));
    let view = p.context_view("t").unwrap();
    assert_eq!(view["retrieval_method"], "merged-path-symbol-diagnostic-v2");
    let db = Connection::open(p.state.join("project.db")).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM index_meta", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn scalar_text_edits_preserve_partial_crlf_ranges_and_never_unescape_literals() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    std::fs::write(
        cwd.path().join("a.py"),
        "# ☃\r\ndef calc():\r\n return 1\r\n# tail\r\n",
    )
    .unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    p.start_task("t", "repair").unwrap();
    p.note("t", "plan", "Patch then check", "user").unwrap();
    let read = p.read("t", "a.py", 2, 2).unwrap();
    let c = p
        .prepare_text_edit(
            "t",
            "a.py",
            read["range_handle"].as_str().unwrap(),
            "def calc():\n return '\\n☃'",
            "Scalar source",
        )
        .unwrap();
    p.apply(&c.id, Policy::Trusted).unwrap();
    assert_eq!(
        std::fs::read_to_string(cwd.path().join("a.py")).unwrap(),
        "# ☃\r\ndef calc():\r\n return '\\n☃'\r\n# tail\r\n"
    );
    let tools = toolbox::focused_tools(ToolProfile::Compact);
    let names: Vec<_> = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(
        names.contains(&"edit_text")
            && names.contains(&"create_file")
            && !names.contains(&"edit_lines")
    );
    let edit = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "edit_text")
        .unwrap();
    assert_eq!(
        edit["inputSchema"]["required"],
        serde_json::json!(["path", "handle", "new_text"])
    );
}

#[test]
fn edit_feedback_refreshes_only_actual_changed_source_for_follow_up_repairs() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    std::fs::write(cwd.path().join("a.py"), "def calc(x):\n return x * 2\n").unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    p.start_task("t", "repair").unwrap();
    p.note("t", "plan", "Patch then check", "user").unwrap();
    let read = p.read("t", "a.py", 1, 100).unwrap();
    let c = p
        .prepare_lines_edit(
            "t",
            "a.py",
            read["range_handle"].as_str(),
            &strings(&["def calc(x):", " return x * 5"]),
            "handle",
            "Initial patch",
        )
        .unwrap();
    let applied = p.apply(&c.id, Policy::Trusted).unwrap();
    let updated = p.changed_read("t", &applied, 1500).unwrap().unwrap();
    assert_eq!(updated["source"], " return x * 5\n");
    assert_eq!(updated["start_line"], 2);
    assert_ne!(updated["sha256"], read["sha256"]);
    let c = p
        .prepare_lines_edit(
            "t",
            "a.py",
            updated["range_handle"].as_str(),
            &strings(&[" return x * 3"]),
            "handle",
            "Repair from actual diagnostics",
        )
        .unwrap();
    p.apply(&c.id, Policy::Trusted).unwrap();
    assert_eq!(
        std::fs::read_to_string(cwd.path().join("a.py")).unwrap(),
        "def calc(x):\n return x * 3\n"
    );
}

#[test]
fn compact_tools_keep_terminal_checks_and_skills_without_ambiguous_edit_schemas() {
    let tools = toolbox::focused_tools(ToolProfile::CompactLines);
    let names: Vec<_> = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"edit_lines"));
    let edit = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "edit_lines")
        .unwrap();
    assert!(
        edit["inputSchema"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f == "handle")
    );
    assert!(edit["inputSchema"]["properties"].get("operation").is_none());
    assert!(!names.contains(&"edit"));
    for name in ["terminal", "run_check", "skill", "read", "evidence"] {
        assert!(names.contains(&name));
    }
    let tools = toolbox::focused_tools(ToolProfile::All);
    assert!(
        !tools["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "edit_lines")
    );
}

#[tokio::test]
async fn compact_context_prioritizes_actual_failures_over_historical_claims() {
    use std::sync::{Arc, atomic::AtomicBool};
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    std::fs::write(cwd.path().join("calc.py"), "def calc(x): return x\n").unwrap();
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        p.start_task("t", "calc repair").unwrap();
        p.note("t", "plan", "Patch then check", "user").unwrap();
        p.set_check(&alt_cli::project::CheckSpec {
            name: "behavior".into(),
            argv: vec![
                "python3".into(),
                "-c".into(),
                "print('ACTUAL_DIAGNOSTIC_731'); raise SystemExit(1)".into(),
            ],
            timeout_secs: 10,
            contract: Default::default(),
        })
        .unwrap();
        p.note(
            "t",
            "decision",
            "Cedar calc must use the bundled helper",
            "user",
        )
        .unwrap();
        for n in 0..20 {
            p.note("t", "decision", &format!("Unrelated layout {n}"), "user")
                .unwrap();
        }
        p.note(
            "t",
            "hypothesis",
            "All tests definitely pass",
            "model note (unverified)",
        )
        .unwrap();
    }
    let check = alt_cli::sandbox::run(
        data.path().into(),
        cwd.path().into(),
        "t".into(),
        "behavior".into(),
        Policy::Trusted,
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .unwrap();
    assert_eq!(check.exit_code, Some(1));
    let mut p = Project::open(data.path(), cwd.path()).unwrap();
    let memory = p.compact_memory("t", "Cedar calc", 2400).unwrap();
    assert!(memory.starts_with("Current check status: Check failed"));
    assert!(memory.contains("ACTUAL_DIAGNOSTIC_731"));
    assert!(memory.contains(&check.id));
    assert!(memory.contains("Cedar calc must use the bundled helper"));
    assert!(memory.find("ACTUAL_DIAGNOSTIC_731") < memory.find("All tests definitely pass"));
    assert!(!p.verification().unwrap().behavioral_acceptance);
}
