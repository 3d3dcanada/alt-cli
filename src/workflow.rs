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
    let diagnostic=check.map(|c| {
        let locations:Vec<_>=c.output.lines().filter(|l|l.contains("File \"")||l.trim_start().starts_with("-->")||l.contains("AssertionError")||l.contains("error[")||l.contains("FAILED")||l.contains("Expected:")||l.contains("Received:")||l.contains("expected")||l.contains("actual")).take(12).map(|l|crate::project::bounded(l,320)).collect();
        json!({"evidence_id":c.id,"source_revision":c.snapshot,"current_source":Some(&c.snapshot)==source.as_ref(),"name":c.name,"exit_code":c.exit_code,"timed_out":c.timed_out,"cancelled":c.cancelled,"error":c.error,"tests_run":c.tests_run,"observed_lines":locations,"output_preview":crate::project::bounded(&c.output,1600),"captured_output_truncated":c.output_truncated,"raw_reference":format!("task evidence/export: check {}",c.id),"scope":"Locations and expected/actual strings are quoted from the actual output. Missing values remain unknown."})
    });
    let mut stmt=p.db.prepare("SELECT body,source FROM notes WHERE task=?1 AND kind='hypothesis' ORDER BY seq DESC LIMIT 3")?;
    let hypotheses=stmt.query_map([task],|r|Ok(json!({"text":r.get::<_,String>(0)?,"source":r.get::<_,String>(1)?,"verified":false})))?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(
        json!({"schema":1,"task":task,"goal":state.goal,"stage":stage,"source_revision":source,"next_decision":decision,"facts":{"applied_changes":state.changes,"check_status":state.check_status,"verification":verification,"diagnostic":diagnostic},"hypotheses":hypotheses,"budget_scope":"Provider requests record actual allocations; this packet does not invent consumed tokens."}),
    )
}
