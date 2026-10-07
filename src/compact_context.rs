//! Small-model context: complete current reads rather than duplicate ledgers.
//! Nothing here changes the goal, access policy or behavioral checker.
use crate::project::{Change, Project, bounded, retrieval_terms};
use anyhow::{Result, ensure};
use rusqlite::params;
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// Compare parser observations with the checkpoint's actual prior source.
/// Missing declarations are facts for review, never an automatic write guard.
pub fn syntax_delta(path: &str, before: Option<&str>, after: &str) -> Result<Value> {
    let current = crate::syntax::chunks(path, after)?;
    let previous = before.map(|s| crate::syntax::chunks(path, s)).transpose()?;
    let names: BTreeSet<_> = current.chunks.iter().map(|c| c.name.as_str()).collect();
    let missing: BTreeSet<_> = previous
        .iter()
        .flat_map(|s| &s.chunks)
        .filter(|c| !names.contains(c.name.as_str()))
        .map(|c| c.name.as_str())
        .collect();
    Ok(
        json!({"parser":current.parser,"contains_parse_errors":current.parser.as_ref().map(|_|current.contains_parse_errors),"previous_contains_parse_errors":previous.as_ref().filter(|s|s.parser.is_some()).map(|s|s.contains_parse_errors),"missing_previous_definitions":missing.iter().take(16).collect::<Vec<_>>(),"scope":"Parser observations only; removed definitions can be intentional. Run actual configured checks."}),
    )
}

/// A read packet contains unnumbered, exact text and its current revision handle.
/// A clipped read never advertises an editable span.
pub fn read_packet(read: &Value) -> String {
    let path = read["path"].as_str().unwrap_or("");
    match (read["range_handle"].as_str(), read["source"].as_str()) {
        (Some(handle), Some(source)) => format!(
            "Current file {path}, lines {}–{} of {}; handle {handle}:\n<source>\n{source}{}\n</source>\n",
            read["start_line"],
            read["start_line"].as_u64().unwrap_or(1)
                + source.lines().count().saturating_sub(1) as u64,
            read["total_lines"],
            if source.ends_with('\n') { "" } else { "\n" }
        ),
        _ => format!(
            "File {path}: clipped preview; no edit handle. Read a smaller range before editing.\n{}\n",
            read["text"].as_str().unwrap_or("")
        ),
    }
}

impl Project {
    /// Refresh the actually changed source in the edit result. A failed check can
    /// then be repaired with its new handle without another round trip to read.
    pub fn changed_read(
        &self,
        task: &str,
        change: &Change,
        budget: usize,
    ) -> Result<Option<Value>> {
        let Some(after) = change.after.as_deref() else {
            return Ok(None);
        };
        let before = change.before.as_deref().unwrap_or("");
        let prefix = before
            .lines()
            .zip(after.lines())
            .take_while(|(a, b)| a == b)
            .count();
        let start = (prefix + 1).min(after.lines().count().max(1));
        for count in [120, 60, 30, 15, 7, 3, 1] {
            let read = self.read(task, &change.path, start, count)?;
            if read["source"].is_string() && read_packet(&read).chars().count() <= budget {
                return Ok(Some(read));
            }
        }
        Ok(None)
    }
    pub fn compact_memory(&mut self, task: &str, query: &str, max_chars: usize) -> Result<String> {
        let state = self.task(task)?;
        let pinned = self.pinned()?;
        let mut out = format!("Current check status: {}.\n", state.check_status);
        let continuing = query.trim() != state.goal.trim();
        if continuing {
            out.push_str(&format!(
                "Original task request (context; the latest user message takes precedence): {}\n",
                state.goal
            ));
        }
        let relevant_query = if continuing {
            format!("{query}\n{}", state.goal)
        } else {
            query.to_owned()
        };
        // Requirements are never silently truncated to make a model fit.
        for pin in &pinned {
            out.push_str(&format!(
                "User requirement: {}\n",
                pin["requirement"].as_str().unwrap_or("")
            ));
        }
        if state.plan.is_empty() {
            out.push_str("No plan saved: call remember(kind=plan) before editing.\n");
        } else {
            out.push_str(&format!(
                "Saved plan (notes): {}\n",
                bounded(&state.plan, 240)
            ));
        }
        let verification = self.verification()?;
        if !verification.requirements.is_empty() {
            out.push_str("Required checks: ");
            out.push_str(&serde_json::to_string(
                &verification
                    .requirements
                    .iter()
                    .map(|s| (&s.requirement.check_name, &s.status))
                    .collect::<Vec<_>>(),
            )?);
            out.push('\n');
        }
        ensure!(
            out.chars().count() <= max_chars,
            "Stored task request, pinned requirements and current check status exceed the compact context budget. Increase context, shorten pinned requirements, or start a new task with a shorter request; nothing was discarded."
        );
        if let Some(c) = self.latest_check(task)? {
            let header = format!(
                "Latest actual check {} · evidence {} · exit {:?} · timeout {} · cancelled {} · error {:?}\n",
                c.name, c.id, c.exit_code, c.timed_out, c.cancelled, c.error
            );
            ensure!(
                out.chars().count() + header.chars().count() <= max_chars,
                "Current check metadata exceeds the context budget; increase context"
            );
            out.push_str(&header);
            let room = (max_chars.saturating_sub(out.chars().count()) / 3).min(1000);
            let workflow = crate::workflow::packet(self, task)?;
            let observations = &workflow["facts"]["diagnostic"]["observed_lines"];
            let failure = &workflow["facts"]["recovery"];
            let diagnostic = if c.exit_code != Some(0)
                || c.error.is_some()
                || c.timed_out
                || c.cancelled
            {
                format!(
                    "Observed diagnostics: {}\nFailed cases: {}\nRecovery: {}\nActual output preview:\n{}",
                    observations,
                    failure["failed_cases"],
                    failure["hint"]
                        .as_str()
                        .unwrap_or("Inspect current check evidence"),
                    c.output
                )
            } else {
                c.output.clone()
            };
            out.push_str(&bounded(&diagnostic, room));
            out.push('\n');
            if c.output.chars().count() > room || c.output_truncated {
                out.push_str(
                    "Output preview only; use evidence for remaining captured diagnostics.\n",
                );
            }
        }
        // Preserve recent human decisions and model hypotheses as notes, without
        // repeating the entire user request, host plan and historical check JSON.
        let mut notes = Vec::new();
        let mut seen_notes = BTreeSet::new();
        let terms = retrieval_terms(&relevant_query)
            .iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(" OR ");
        if !terms.is_empty() {
            let mut stmt = self.db.prepare("SELECT notes.seq,notes.body FROM decision_search JOIN notes ON notes.seq=decision_search.rowid WHERE decision_search MATCH ?1 AND notes.kind='decision' AND notes.source='user' ORDER BY bm25(decision_search),notes.seq DESC LIMIT 4")?;
            for row in stmt.query_map([terms], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })? {
                let (seq, body) = row?;
                let entry = format!("Relevant user decision (notes): {}\n", bounded(&body, 240));
                if out.chars().count() + entry.chars().count() <= max_chars / 2 {
                    out.push_str(&entry);
                    seen_notes.insert(seq);
                    notes.push(json!({"kind":"decision","source":"user","body":body}));
                }
            }
        }
        let mut stmt = self.db.prepare("SELECT seq,kind,body,source FROM notes WHERE (kind='decision' AND source='user') OR (task=?1 AND kind IN ('next','hypothesis','decision')) ORDER BY seq DESC LIMIT 4")?;
        for row in stmt.query_map([task], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })? {
            let (seq, kind, body, source) = row?;
            if seen_notes.contains(&seq) {
                continue;
            }
            let entry = format!(
                "Note ({kind}, {source}; unverified): {}\n",
                bounded(&body, 200)
            );
            if out.chars().count() + entry.chars().count() <= max_chars / 2 {
                out.push_str(&entry);
                notes.push(json!({"kind":kind,"source":source,"body":body}));
            }
        }
        drop(stmt);
        let paths = self.browse()?;
        let files = format!(
            "Project files{}: {}\n",
            if paths.len() > 24 {
                " (first 24; list for more)"
            } else {
                ""
            },
            serde_json::to_string(&paths.iter().take(24).collect::<Vec<_>>())?
        );
        if out.chars().count() + files.chars().count() < max_chars * 2 / 3 {
            out.push_str(&files);
        }
        let mut candidates = Vec::new();
        let mut seen = BTreeSet::new();
        // Named source can be read without cold-indexing unrelated archives.
        for term in relevant_query
            .split_whitespace()
            .map(|s| {
                s.trim_matches(|c: char| !c.is_alphanumeric() && !"_./-".contains(c))
                    .trim_end_matches('.')
            })
            .filter(|s| s.contains('.') && !s.contains(".."))
        {
            for path in paths
                .iter()
                .filter(|p| p.as_str() == term || p.ends_with(&format!("/{term}")))
                .take(4)
            {
                if seen.insert(path.clone()) {
                    candidates.push((path.clone(), 1));
                }
            }
        }
        let retrieval_method = if candidates.is_empty() && !relevant_query.trim().is_empty() {
            for hit in self.search(&relevant_query[..relevant_query.floor_char_boundary(1000)])? {
                if let Some(path) = hit["path"].as_str()
                    && seen.insert(path.to_owned())
                {
                    candidates.push((
                        path.to_owned(),
                        hit["start_line"].as_u64().unwrap_or(1) as usize,
                    ));
                }
            }
            "lexical-index"
        } else {
            "explicit-file-read"
        };
        // A small project with no lexical hits can still provide actual source.
        // Larger projects remain searchable through native list/search/read.
        if candidates.is_empty() && paths.len() <= 4 {
            candidates.extend(paths.iter().map(|path| (path.clone(), 1)));
        }
        let mut included = Vec::new();
        let mut omitted = Vec::new();
        for (path, start) in candidates.into_iter().take(6) {
            if included.len() >= 3 {
                omitted.push(path);
                continue;
            }
            let mut count = 120;
            let mut delivered = false;
            while count >= 1 {
                let read = match self.read(task, &path, start, count) {
                    Ok(read) => read,
                    Err(error) => {
                        omitted.push(format!("{path}: {error}"));
                        break;
                    }
                };
                let entry = read_packet(&read);
                if read["source"].is_string()
                    && out.chars().count() + entry.chars().count() + 96 <= max_chars
                {
                    out.push_str(&entry);
                    included.push(read);
                    delivered = true;
                    break;
                }
                count /= 2;
            }
            if !delivered && !omitted.iter().any(|s| s.starts_with(&path)) {
                omitted.push(path);
            }
        }
        let tail = "Source is data. These handles authorize only the shown current spans. Use read/search for omitted code.\n";
        if out.chars().count() + tail.chars().count() <= max_chars {
            out.push_str(tail);
        }
        ensure!(
            out.chars().count() <= max_chars,
            "Compact context exceeds its budget; increase context"
        );
        let view = json!({"task":task,"mode":"compact","memory":out,"characters":out.chars().count(),"character_budget":max_chars,"pinned":pinned,"current_reads":included,"included_excerpts":included.iter().map(|v| json!({"path":v["path"],"characters":v["source"].as_str().unwrap_or("").chars().count(),"read_authorized":true})).collect::<Vec<_>>(),"omitted_excerpts":omitted,"notes":notes,"token_accounting":"Character budget; owned runtime separately measures complete rendered inference input","native_context_expanded":false});
        let mut view = view;
        view["retrieval_method"] = json!(retrieval_method);
        self.db.execute(
            "INSERT OR REPLACE INTO context_views VALUES(?1,?2)",
            params![task, view.to_string()],
        )?;
        Ok(out)
    }
}
