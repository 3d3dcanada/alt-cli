//! A06: small-model edit robustness: whole-file rewrite and forgiving old_text.
use alt_cli::project::{Policy, Project};
use std::fs;

fn begin(p: &Project) {
    p.start_task("task", "Repair the function").unwrap();
    p.note("task", "plan", "Read, edit, check", "user").unwrap();
}

const SOURCE: &str = "def total(items):\n    result = 0\n    for item in items:\n        result += item\n    return result\n\n\ndef mean(items):\n    return total(items) / len(items)\n";

#[test]
fn replace_file_needs_a_complete_current_read_and_a_small_file() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    fs::write(cwd.path().join("stats.py"), SOURCE).unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    begin(&p);
    let rewritten = "def total(items):\n    return sum(items)\n\n\ndef mean(items):\n    return total(items) / len(items) if items else 0.0\n";
    // A partial read is not enough for a whole-file rewrite.
    p.read("task", "stats.py", 1, 3).unwrap();
    let partial = p
        .prepare_replace_file("task", "stats.py", rewritten, "Simplify")
        .unwrap_err()
        .to_string();
    assert!(partial.contains("Read the whole current file"), "{partial}");
    p.read("task", "stats.py", 1, 300).unwrap();
    let change = p
        .prepare_replace_file("task", "stats.py", rewritten, "Simplify")
        .unwrap();
    p.apply(&change.id, Policy::Guided).unwrap();
    assert_eq!(
        fs::read_to_string(cwd.path().join("stats.py")).unwrap(),
        rewritten
    );
    p.undo(&change.id).unwrap();
    assert_eq!(
        fs::read_to_string(cwd.path().join("stats.py")).unwrap(),
        SOURCE
    );
    // Larger files must use span edits.
    let big: String = (0..301).map(|i| format!("x{i} = {i}\n")).collect();
    fs::write(cwd.path().join("big.py"), &big).unwrap();
    p.read("task", "big.py", 1, 300).unwrap();
    let error = p
        .prepare_replace_file("task", "big.py", "x = 1\n", "Rewrite")
        .unwrap_err()
        .to_string();
    assert!(error.contains("up to 300 lines"), "{error}");
}

#[test]
fn old_text_matches_exactly_then_ignoring_whitespace_and_reindents() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    fs::write(cwd.path().join("stats.py"), SOURCE).unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    begin(&p);
    p.read("task", "stats.py", 1, 300).unwrap();
    // The model lost the indentation of the loop body.
    let old = "for item in items:\n    result += item";
    let new = "for item in items:\n    if item is not None:\n        result += item";
    let change = p
        .prepare_edit("task", "stats.py", None, old, new, "replace", "Skip None")
        .unwrap();
    let note = change.matched.clone().unwrap();
    assert!(
        note.contains("lines 3–4") && note.contains("re-indented"),
        "{note}"
    );
    let after = change.after.clone().unwrap();
    assert!(
        after.contains(
            "    for item in items:\n        if item is not None:\n            result += item\n"
        ),
        "{after}"
    );
    // An exact match stays exact and records nothing.
    p.apply(&change.id, Policy::Guided).unwrap();
    p.read("task", "stats.py", 1, 300).unwrap();
    let exact = p
        .prepare_edit(
            "task",
            "stats.py",
            None,
            "return result",
            "return int(result)",
            "replace",
            "Cast",
        )
        .unwrap();
    assert!(exact.matched.is_none());
}

#[test]
fn a_miss_returns_the_closest_current_span_with_a_usable_handle() {
    let data = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    fs::write(cwd.path().join("stats.py"), SOURCE).unwrap();
    let p = Project::open(data.path(), cwd.path()).unwrap();
    begin(&p);
    p.read("task", "stats.py", 1, 300).unwrap();
    let error = p
        .prepare_edit(
            "task",
            "stats.py",
            None,
            "def mean(values):\n    return total(values) / len(values)",
            "def mean(values):\n    return 0.0",
            "replace",
            "Guard empty input",
        )
        .unwrap_err()
        .to_string();
    assert!(error.contains("Closest current text (lines 8–9"), "{error}");
    assert!(error.contains("def mean(items):"), "{error}");
    let handle = error
        .split("handle ")
        .nth(1)
        .and_then(|rest| rest.split(')').next())
        .unwrap()
        .to_owned();
    // The returned handle edits exactly that span.
    let change = p
        .prepare_handle_edit(
            "task",
            "stats.py",
            &handle,
            "def mean(items):\n    return total(items) / len(items) if items else 0.0",
            "Guard empty input",
        )
        .unwrap();
    assert!(change.after.unwrap().ends_with("if items else 0.0\n"));
    // Ambiguous text is refused rather than guessed.
    fs::write(cwd.path().join("twice.py"), "x = 1\n\nx = 1\n").unwrap();
    p.read("task", "twice.py", 1, 300).unwrap();
    let twice = p
        .prepare_edit(
            "task", "twice.py", None, "  x = 1", "x = 2", "replace", "Bump",
        )
        .unwrap_err()
        .to_string();
    assert!(twice.contains("more than one place"), "{twice}");
}
