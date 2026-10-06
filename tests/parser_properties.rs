use alt_cli::{
    language,
    project::Project,
    verification::{self, Format},
};
#[test]
fn bounded_mutations_of_reports_paths_and_utf16_positions_never_panic() {
    let mut random = 0x5445535453454544u64;
    let seeds = [
        br#"{"schema":1,"complete":true,"tests":[{"name":"one","status":"passed"}]}"#.as_slice(),
        b"<testsuite tests=\"1\"><testcase name=\"one\"/></testsuite>",
        b"TAP version 13\n1..1\nok 1 - one\n",
    ];
    for n in 0..12_000 {
        let mut bytes = seeds[n % 3].to_vec();
        for _ in 0..1 + n % 8 {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            let index = random as usize % bytes.len();
            bytes[index] = (random >> 32) as u8;
        }
        let outcome =
            verification::parse([Format::Json, Format::Junit, Format::Tap][n % 3], &bytes);
        if let Ok(o) = outcome {
            assert!(o.complete);
            assert_eq!(o.executed, o.passed + o.failed);
            assert!(o.executed + o.skipped <= 100_000);
        }
        let path = String::from_utf8_lossy(&bytes);
        let _ = Project::validate_path(&path);
        for line in [0, 1, 2, u64::MAX] {
            if let Ok(offset) = language::offset(&path, line, random % 256) {
                assert!(offset <= path.len() && path.is_char_boundary(offset));
            }
        }
    }
    let text = "a🙂é\nb雪";
    assert_eq!(language::offset(text, 0, 1).unwrap(), 1);
    assert!(language::offset(text, 0, 2).is_err());
    assert_eq!(language::offset(text, 0, 3).unwrap(), 5);
    assert_eq!(language::offset(text, 1, 1).unwrap(), 9);
    for bad in ["../escape", "/absolute", "a/../../escape", "a\0b"] {
        assert!(Project::validate_path(bad).is_err());
    }
}
#[test]
fn explicit_tool_focus_retains_full_access_terminal_option() {
    use alt_cli::toolbox::{ToolProfile, focused_tools};
    let names = |p| {
        focused_tools(p)["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["name"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    };
    assert!(names(ToolProfile::All).contains(&"terminal".into()));
    assert!(names(ToolProfile::Terminal).contains(&"terminal".into()));
    assert!(!names(ToolProfile::Coding).contains(&"terminal".into()));
    assert!(names(ToolProfile::Coding).contains(&"edit".into()));
    assert!(!names(ToolProfile::Inspect).contains(&"edit".into()));
}
