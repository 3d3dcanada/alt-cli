//! Host decisions use recorded source/check state. Model hypotheses remain unverified notes.
use crate::project::Project;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    #[default]
    ModelPlan,
    Host,
}

pub fn begin(p: &Project, task: &str, mode: Mode) -> Result<()> {
    if matches!(mode, Mode::Host) && p.task(task)?.plan.is_empty() {
        p.note(task,"plan","Host workflow: inspect current source; reproduce with configured checks; patch using revision-bound handles; rerun required checks; recover from actual diagnostics.","host workflow (not a model-authored plan)")?;
    }
    Ok(())
}
pub fn packet(p: &Project, task: &str) -> Result<Value> {
    let state = p.task(task)?;
    let check = p.latest_check(task)?;
    let source = if check.is_some() || state.changes > 0 {
        p.snapshot().ok().map(|s| s.0)
    } else {
        None
    };
    let verification = p.verification()?;
    let stage = if state.plan.is_empty() {
        "plan"
    } else if check.as_ref().is_some_and(|c| c.cancelled) {
        "recover"
    } else if verification.complete && verification.behavioral_acceptance {
        "review"
    } else if check
        .as_ref()
        .is_some_and(|c| Some(&c.snapshot) != source.as_ref())
    {
        "check"
    } else if check
        .as_ref()
        .is_some_and(|c| c.exit_code != Some(0) || c.error.is_some() || c.timed_out)
    {
        "recover"
    } else if state.changes > 0 {
        "check"
    } else {
        "inspect"
    };
    let recovery = recovery(p, task)?;
    let decision = match stage {
        "plan" => "Save a brief model plan or explicitly select host workflow.",
        "recover" => {
            "Inspect the observed failure at its current source location; change the hypothesis or implementation before repeating."
        }
        "check" => {
            "Run the configured checks on the current revision; earlier evidence cannot verify this revision."
        }
        "review" => {
            "Required independent checks pass on current source; review the diff and describe their scope."
        }
        _ => {
            "Read the relevant implementation and run a configured check to reproduce the behavior."
        }
    };
    let decision = if stage == "recover" {
        recovery
            .as_ref()
            .and_then(|r| r["hint"].as_str())
            .unwrap_or(decision)
    } else {
        decision
    };
    let source_locations = check
        .as_ref()
        .map(|c| diagnostic_locations(p, c))
        .transpose()?
        .unwrap_or_default();
    let failure_packet = check.as_ref().map(|c| failure_packet(p, c)).transpose()?;
    let diagnostic=check.map(|c| {
        let locations=failure_packet.as_ref().map(|v|v["observed_diagnostics"].clone()).unwrap_or(json!([]));
        json!({"evidence_id":c.id,"source_revision":c.snapshot,"current_source":Some(&c.snapshot)==source.as_ref(),"name":c.name,"exit_code":c.exit_code,"timed_out":c.timed_out,"cancelled":c.cancelled,"error":c.error,"tests_run":c.tests_run,"observed_lines":locations,"source_locations":source_locations,"output_preview":failure_packet.as_ref().map(|v|v["captured_output_preview"].clone()).unwrap_or_else(||json!(crate::project::bounded(&c.output,1600))),"captured_output_truncated":c.output_truncated,"raw_reference":format!("task evidence/export: check {}",c.id),"scope":"Locations and expected/actual strings are quoted from the actual output. Missing values remain unknown."})
    });
    let mut stmt=p.db.prepare("SELECT body,source FROM notes WHERE task=?1 AND kind='hypothesis' ORDER BY seq DESC LIMIT 3")?;
    let hypotheses=stmt.query_map([task],|r|Ok(json!({"text":r.get::<_,String>(0)?,"source":r.get::<_,String>(1)?,"verified":false})))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(
        json!({"schema":1,"task":task,"goal":state.goal,"stage":stage,"source_revision":source,"next_decision":decision,"facts":{"applied_changes":state.changes,"check_status":state.check_status,"verification":verification,"diagnostic":diagnostic,"failure_packet":failure_packet,"recovery":recovery},"hypotheses":hypotheses,"budget_scope":"Provider requests record actual allocations; this packet does not invent consumed tokens."}),
    )
}

fn failed(c: &crate::project::CheckResult) -> bool {
    c.exit_code != Some(0) || c.error.is_some() || c.timed_out || c.cancelled
}

/// Names quoted from integrity-checked saved reports. These are observations,
/// not expected implementations or a model's proposed explanation.
fn failed_cases(p: &Project, c: &crate::project::CheckResult) -> Vec<String> {
    use crate::verification::Format;
    let Some(outcome) = &c.structured else {
        return Vec::new();
    };
    if !c.id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        return Vec::new();
    }
    let path = p
        .state
        .join("check-reports")
        .join(format!("{}.report", c.id));
    use std::io::Read;
    let Ok(file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let mut bytes = Vec::new();
    if file
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return Vec::new();
    }

    if bytes.len() > 8 * 1024 * 1024 || crate::project::digest(&bytes) != outcome.report_sha256 {
        return Vec::new();
    }
    let mut names: Vec<String> = match outcome.format {
        Format::Json => serde_json::from_slice::<Value>(&bytes)
            .ok()
            .and_then(|v| {
                v["tests"].as_array().map(|rows| {
                    rows.iter()
                        .filter(|r| r["status"] == "failed")
                        .filter_map(|r| r["name"].as_str().map(str::to_owned))
                        .collect()
                })
            })
            .unwrap_or_default(),
        Format::Junit => std::str::from_utf8(&bytes)
            .ok()
            .and_then(|text| {
                roxmltree::Document::parse(text).ok().map(|doc| {
                    doc.descendants()
                        .filter(|n| {
                            n.has_tag_name("testcase")
                                && n.children()
                                    .any(|c| c.has_tag_name("failure") || c.has_tag_name("error"))
                        })
                        .map(|n| {
                            format!(
                                "{}::{}",
                                n.attribute("classname").unwrap_or(""),
                                n.attribute("name").unwrap_or("")
                            )
                        })
                        .collect()
                })
            })
            .unwrap_or_default(),
        Format::Tap => String::from_utf8_lossy(&bytes)
            .lines()
            .filter(|l| l.trim_start().starts_with("not ok "))
            .map(str::to_owned)
            .collect(),
    };
    names.sort();
    names
}

fn observed_lines(c: &crate::project::CheckResult) -> Vec<String> {
    c.output
        .lines()
        .filter(|l| {
            l.contains("File \"")
                || l.trim_start().starts_with("-->")
                || l.contains("AssertionError")
                || l.contains("error[")
                || l.contains("FAILED")
                || l.contains("Expected:")
                || l.contains("Received:")
                || l.contains("expected")
                || l.contains("actual")
                || l.contains("SyntaxError")
                || l.contains("TypeError")
                || l.contains("panicked at ")
                || l.trim_start().starts_with("left:")
                || l.trim_start().starts_with("right:")
                || l.starts_with("ALT_OBSERVATION ")
        })
        .take(12)
        .map(|l| crate::project::bounded(l, 320))
        .collect()
}

fn diagnostic_text(p: &Project, c: &crate::project::CheckResult) -> String {
    if !c.output_truncated {
        return c.output.clone();
    }
    let mut text = String::new();
    for stream in ["stderr", "stdout"] {
        match crate::verification::read_execution_log(p, &c.id, stream, None, 6000) {
            Ok(log) => text.push_str(&format!(
                "Retained {stream} tail, byte offset {} (integrity checked):\n{}\n",
                log["offset"],
                log["text"].as_str().unwrap_or("")
            )),
            Err(error) => text.push_str(&format!("Retained {stream} tail unavailable: {error}\n")),
        }
    }
    text.push_str("Original captured preview:\n");
    text.push_str(&c.output);
    text
}

fn failure_identity(p: &Project, c: &crate::project::CheckResult) -> Result<String> {
    let mut actual = c.clone();
    actual.output = diagnostic_text(p, c);
    let lines = observed_lines(&actual);
    // Temporary copy paths are not a changed diagnosis. Raw paths remain intact
    // in evidence; normalization is used only for this advisory comparison.
    let copies = regex::Regex::new(r#"/(?:tmp|var/tmp)/[^\s\"')]+"#)?;
    let text = if lines.is_empty() {
        crate::project::bounded(&c.output, 2400)
    } else {
        lines.join("\n")
    };
    Ok(crate::project::digest(&serde_json::to_vec(
        &json!({"name":c.name,"argv":c.argv,"exit":c.exit_code,"error":c.error,"timeout":c.timed_out,"cancelled":c.cancelled,"failed_cases":failed_cases(p,c),"observed_diagnostics":copies.replace_all(&text,"<temporary-copy>")}),
    )?))
}

/// Bounded history detects unchanged failures and A/B/A source cycles. It never
/// refuses a command, resets costs, edits checks or asserts a cause is proven.
pub fn recovery(p: &Project, task: &str) -> Result<Option<Value>> {
    let mut query=p.db.prepare("SELECT payload FROM checks WHERE json_extract(payload,'$.task')=?1 ORDER BY rowid DESC LIMIT 6")?;
    let checks = query
        .query_map([task], |r| r.get::<_, String>(0))?
        .map(|row| Ok(serde_json::from_str::<crate::project::CheckResult>(&row?)?))
        .collect::<Result<Vec<_>>>()?;
    let Some(latest) = checks.first().filter(|c| failed(c)) else {
        return Ok(None);
    };
    let fingerprint = failure_identity(p, latest)?;
    let mut same_failure = 0;
    let mut unchanged = 0;
    let mut changed_between = false;
    let mut revisited = false;
    for (index, c) in checks.iter().enumerate() {
        if c.name != latest.name || c.argv != latest.argv || !failed(c) {
            break;
        }
        let same = failure_identity(p, c)? == fingerprint;
        if same {
            same_failure += 1;
        }
        if index == unchanged && c.snapshot == latest.snapshot && same {
            unchanged += 1;
        }
        if c.snapshot != latest.snapshot {
            changed_between = true;
        }
        if index > 0 && changed_between && c.snapshot == latest.snapshot {
            revisited = true;
        }
    }
    let hint = if revisited {
        "This source revision already failed before another revision was tried. Reread the current implementation and compare the earlier diff; save a different hypothesis instead of cycling between the same edits."
    } else if unchanged >= 2 {
        "The same check failed repeatedly on unchanged source. Reread the implementation and actual failed cases, then change the implementation or explain a missing dependency before rerunning."
    } else if same_failure >= 2 {
        "The same observed failure remains after edits. Read the affected code and failed cases; change the hypothesis and make one focused change before rerunning the original checks."
    } else {
        "Read the actual failed cases and current implementation. Make a focused change, then rerun the original checks; a longer response does not establish a repair."
    };
    let cases = failed_cases(p, latest);
    Ok(Some(
        json!({"evidence_id":latest.id,"source_revision":latest.snapshot,"same_observed_failure_count":same_failure,"unchanged_revision_failures":unchanged,"revisited_failed_revision":revisited,"failed_case_count":cases.len(),"failed_cases":cases.iter().take(16).map(|s|crate::project::bounded(s,240)).collect::<Vec<_>>(),"hint":hint,"scope":"Advisory comparison of up to six recent failed checks on this task. Quoted case names and normalized diagnostic identity do not prove a root cause; commands remain available."}),
    ))
}

/// Keep every location's origin. Relative paths from another test crate are
/// observations, never an authorization to edit an identically named local file.
pub fn diagnostic_observations(p: &Project, c: &crate::project::CheckResult) -> Result<Vec<Value>> {
    use std::path::{Component, Path};
    let files = p.browse()?;
    let serialized = serde_json::to_value(c)?;
    let snapshot_root = serialized["execution"]["snapshot_root"].as_str();
    let namespace_root = serialized["execution"]["namespace_root"].as_str();
    let plain = regex::Regex::new(
        r#"(?:File \"([^\"]+)\", line (\d+)|-->\s+(.+?):(\d+):(\d+)|panicked at (.+?):(\d+):(\d+))"#,
    )?;
    let mut rows = Vec::new();
    let mut observe = |raw: &str,
                       line: u64,
                       column: Option<u64>,
                       crate_root: Option<&str>,
                       origin: &str| {
        let raw_path = Path::new(raw);
        let resolved = if raw_path.is_absolute() {
            Some(raw_path.to_path_buf())
        } else {
            crate_root.map(|root| Path::new(root).join(raw_path))
        };
        let mapped = resolved
            .as_ref()
            .filter(|path| !path.components().any(|c| matches!(c, Component::ParentDir)))
            .and_then(|path| {
                [
                    Some(p.root.as_path()),
                    snapshot_root.map(Path::new),
                    namespace_root.map(Path::new),
                ]
                .into_iter()
                .flatten()
                .find_map(|root| path.strip_prefix(root).ok())
                .and_then(|path| path.to_str())
                .filter(|path| files.iter().any(|p| p == *path))
                .map(str::to_owned)
            });
        let item = json!({"raw_path":raw,"path":mapped,"line":line,"column":column,"crate_root":crate_root,"diagnostic_origin":origin,"mapping":if mapped.is_some(){"verified project/snapshot root"}else{"unresolved; no editable handle"},"source_revision":c.snapshot,"scope":"Quoted diagnostic location. Only a known project root authorizes a local source mapping; relative test/dependency paths stay unresolved."});
        if line > 0 && !rows.contains(&item) && rows.len() < 24 {
            rows.push(item);
        }
    };
    let output = diagnostic_text(p, c);
    for line in output.lines() {
        // cargo --message-format=json supplies crate provenance independently of
        // its human rendered diagnostic. Do not infer a root from a filename.
        if let Ok(value) = serde_json::from_str::<Value>(line)
            && value["reason"] == "compiler-message"
        {
            let root = value["manifest_path"]
                .as_str()
                .and_then(|p| Path::new(p).parent())
                .and_then(|p| p.to_str());
            if let Some(spans) = value["message"]["spans"].as_array() {
                for span in spans.iter().filter(|s| s["is_primary"] == true).take(4) {
                    if let (Some(path), Some(line)) =
                        (span["file_name"].as_str(), span["line_start"].as_u64())
                    {
                        observe(
                            path,
                            line,
                            span["column_start"].as_u64(),
                            root,
                            "cargo compiler-message",
                        );
                    }
                }
            }
        }
        for capture in plain.captures_iter(line) {
            let raw = capture
                .get(1)
                .or_else(|| capture.get(3))
                .or_else(|| capture.get(6))
                .map(|s| s.as_str())
                .unwrap_or("");
            let number = capture
                .get(2)
                .or_else(|| capture.get(4))
                .or_else(|| capture.get(7))
                .and_then(|s| s.as_str().parse().ok())
                .unwrap_or(0);
            let column = capture
                .get(5)
                .or_else(|| capture.get(8))
                .and_then(|s| s.as_str().parse().ok());
            observe(
                raw,
                number,
                column,
                None,
                "quoted text; relative root unknown",
            );
        }
    }
    Ok(rows)
}

pub fn diagnostic_locations(p: &Project, c: &crate::project::CheckResult) -> Result<Vec<Value>> {
    Ok(diagnostic_observations(p, c)?
        .into_iter()
        .filter(|v| v["path"].is_string())
        .take(4)
        .collect())
}

/// A bounded observed failure packet. It does not synthesize a diagnosis or an
/// implementation, and retains a reference to the complete captured evidence.
pub fn failure_packet(p: &Project, c: &crate::project::CheckResult) -> Result<Value> {
    let mut actual = c.clone();
    actual.output = diagnostic_text(p, c);
    let mut counterexamples = Vec::new();
    for line in actual
        .output
        .lines()
        .filter_map(|l| l.strip_prefix("ALT_OBSERVATION "))
        .take(8)
    {
        if line.len() > 4096 {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            counterexamples.push(json!({"observed":value,"scope":"Check-emitted observation quoted verbatim; not a reference solution"}));
        }
    }
    Ok(
        json!({"schema":2,"evidence_id":c.id,"source_revision":c.snapshot,"check_name":c.name,"reproduction_argv":c.argv,"exit_code":c.exit_code,"timed_out":c.timed_out,"cancelled":c.cancelled,"error":c.error,"failed_cases":failed_cases(p,c),"observed_diagnostics":observed_lines(&actual),"locations":diagnostic_observations(p,c)?,"counterexamples":counterexamples,"captured_output_preview":crate::project::bounded(&actual.output,2000),"captured_output_truncated":c.output_truncated,"raw_reference":format!("evidence(id={}, stream=stdout/stderr)",c.id),"claim":"Observed execution only; passing source requires the independent verification contract"}),
    )
}
