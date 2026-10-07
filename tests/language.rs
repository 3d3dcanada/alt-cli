use alt_cli::{
    language,
    project::{Policy, Project},
};
use serde_json::json;
#[test]
fn utf16_rename_preview_applies_checkpoints_and_rejects_outside_overlap_staleness() {
    assert_eq!(language::offset("a😀b\nnext", 0, 3).unwrap(), 5);
    assert!(language::offset("a😀b", 0, 2).is_err());
    assert!(language::offset("a\nb", 0, 3).is_err());
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    std::fs::write(cwd.path().join("a.py"), "value = 1\nprint(value)\n").unwrap();
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        p.start_task("rename", "Rename value").unwrap();
    }
    let uri = reqwest::Url::from_file_path(cwd.path().join("a.py"))
        .unwrap()
        .to_string();
    let edit = json!({"changes":{uri.clone():[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":5}},"newText":"count"},{"range":{"start":{"line":1,"character":6},"end":{"line":1,"character":11}},"newText":"count"}]}});
    let changes =
        language::prepare_workspace_edit(data.path(), cwd.path(), "rename", &edit).unwrap();
    assert_eq!(
        std::fs::read_to_string(cwd.path().join("a.py")).unwrap(),
        "value = 1\nprint(value)\n"
    );
    let ids = vec![changes[0].id.clone()];
    language::apply(data.path(), cwd.path(), &ids, Policy::Guided).unwrap();
    assert_eq!(
        std::fs::read_to_string(cwd.path().join("a.py")).unwrap(),
        "count = 1\nprint(count)\n"
    );
    {
        let p = Project::open(data.path(), cwd.path()).unwrap();
        p.undo(&ids[0]).unwrap();
    }
    let changes =
        language::prepare_workspace_edit(data.path(), cwd.path(), "rename", &edit).unwrap();
    std::fs::write(cwd.path().join("a.py"), "manual change\n").unwrap();
    assert!(
        language::apply(
            data.path(),
            cwd.path(),
            &[changes[0].id.clone()],
            Policy::Guided
        )
        .is_err()
    );
    let outside = json!({"changes":{"file:///etc/passwd":[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}},"newText":"x"}]}});
    assert!(language::prepare_workspace_edit(data.path(), cwd.path(), "rename", &outside).is_err());
}

#[test]
fn failed_batch_restores_earlier_files_through_the_journal() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    for name in ["a.py", "b.py"] {
        std::fs::write(cwd.path().join(name), "value = 1\n").unwrap();
    }
    let p = Project::open(data.path(), cwd.path()).unwrap();
    p.start_task("batch", "rename").unwrap();
    p.note("batch", "plan", "Inspect rename check", "fixture")
        .unwrap();
    for name in ["a.py", "b.py"] {
        p.read("batch", name, 1, 20).unwrap();
    }
    let a = p
        .prepare_edit("batch", "a.py", None, "value", "count", "replace", "rename")
        .unwrap();
    let b = p
        .prepare_edit("batch", "b.py", None, "value", "count", "replace", "rename")
        .unwrap();
    // Both proposals are initially valid. The third conflicts after the first write.
    let conflict = p
        .prepare_edit(
            "batch",
            "a.py",
            None,
            "value",
            "other",
            "replace",
            "overlapping batch",
        )
        .unwrap();
    drop(p);
    let err = language::apply(
        data.path(),
        cwd.path(),
        &[a.id.clone(), b.id.clone(), conflict.id],
        Policy::Trusted,
    )
    .unwrap_err();
    assert!(err.to_string().contains("rollback"), "{err:#}");
    for name in ["a.py", "b.py"] {
        assert_eq!(
            std::fs::read_to_string(cwd.path().join(name)).unwrap(),
            "value = 1\n"
        );
    }
    let p = Project::open(data.path(), cwd.path()).unwrap();
    assert_eq!(p.change(&a.id).unwrap().status, "undone");
    assert_eq!(p.change(&b.id).unwrap().status, "undone");
}
