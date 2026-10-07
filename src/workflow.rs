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
    let diagnostic=check.map(|c| {
        let locations=observed_lines(&c);
        json!({"evidence_id":c.id,"source_revision":c.snapshot,"current_source":Some(&c.snapshot)==source.as_ref(),"name":c.name,"exit_code":c.exit_code,"timed_out":c.timed_out,"cancelled":c.cancelled,"error":c.error,"tests_run":c.tests_run,"observed_lines":locations,"source_locations":source_locations,"output_preview":crate::project::bounded(&c.output,1600),"captured_output_truncated":c.output_truncated,"raw_reference":format!("task evidence/export: check {}",c.id),"scope":"Locations and expected/actual strings are quoted from the actual output. Missing values remain unknown."})
    });
    let mut stmt=p.db.prepare("SELECT body,source FROM notes WHERE task=?1 AND kind='hypothesis' ORDER BY seq DESC LIMIT 3")?;
    let hypotheses=stmt.query_map([task],|r|Ok(json!({"text":r.get::<_,String>(0)?,"source":r.get::<_,String>(1)?,"verified":false})))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(
        json!({"schema":1,"task":task,"goal":state.goal,"stage":stage,"source_revision":source,"next_decision":decision,"facts":{"applied_changes":state.changes,"check_status":state.check_status,"verification":verification,"diagnostic":diagnostic,"recovery":recovery},"hypotheses":hypotheses,"budget_scope":"Provider requests record actual allocations; this packet does not invent consumed tokens."}),
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
        })
        .take(12)
        .map(|l| crate::project::bounded(l, 320))
        .collect()
}

fn failure_identity(p: &Project, c: &crate::project::CheckResult) -> Result<String> {
    let lines = observed_lines(c);
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

pub fn diagnostic_locations(p: &Project, c: &crate::project::CheckResult) -> Result<Vec<Value>> {
    let files = p.browse()?;
    let expression =
        regex::Regex::new(r#"(?:File \"([^\"]+)\", line (\d+)|-->\s+(.+?):(\d+):(\d+))"#)?;
    let mut locations = Vec::new();
    for capture in expression.captures_iter(&c.output) {
        let raw = capture
            .get(1)
            .or_else(|| capture.get(3))
            .map(|m| m.as_str())
            .unwrap_or("");
        let candidates = files
            .iter()
            .filter(|path| raw == path.as_str() || raw.ends_with(&format!("/{path}")))
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            continue;
        }
        let line = capture
            .get(2)
            .or_else(|| capture.get(4))
            .and_then(|m| m.as_str().parse::<u32>().ok());
        let Some(line) = line.filter(|n| *n > 0) else {
            continue;
        };
        let item = json!({"path":candidates[0],"line":line,"column":capture.get(5).and_then(|m|m.as_str().parse::<u32>().ok()),"source_revision":c.snapshot,"scope":"Location quoted from actual diagnostics and uniquely matched to a repository path; current source may differ from the checked snapshot."});
        if !locations.contains(&item) {
            locations.push(item);
        }
        if locations.len() == 4 {
            break;
        }
    }
    Ok(locations)
}
