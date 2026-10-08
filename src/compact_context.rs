//! Small-model context: complete current reads rather than duplicate ledgers.
//! Nothing here changes the goal, access policy or behavioral checker.
use crate::project::{Change, Project, bounded, retrieval_terms};
use anyhow::{Result, ensure};
use rusqlite::params;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Compare parser observations with the checkpoint's actual prior source.
/// Missing declarations are parser observations; deliberate rewrites remain available.
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

/// Inspect the proposed bytes before applying. Parser uncertainty is explicit;
/// callers may deliberately accept deletion/renaming or incomplete intermediate code.
pub fn edit_preview(change: &Change) -> Result<Value> {
    let syntax = change
        .after
        .as_deref()
        .map(|body| syntax_delta(&change.path, change.before.as_deref(), body))
        .transpose()?;
    let mut concerns = Vec::new();
    if let Some(s) = &syntax {
        if s["contains_parse_errors"] == true && s["previous_contains_parse_errors"] != true {
            concerns.push("proposed source introduces parser errors");
        }
        if s["missing_previous_definitions"]
            .as_array()
            .is_some_and(|v| !v.is_empty())
        {
            concerns
                .push("previous definitions disappear (may be an intentional rename or deletion)");
        }
    }
    if let (Some(before), Some(after)) = (&change.before, &change.after) {
        let before = before.lines().collect::<Vec<_>>();
        let after = after.lines().collect::<Vec<_>>();
        let prefix = before
            .iter()
            .zip(&after)
            .take_while(|(a, b)| a == b)
            .count();
        let suffix = before[prefix..]
            .iter()
            .rev()
            .zip(after[prefix..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        let removed = before.len() - prefix - suffix;
        let inserted = after.len() - prefix - suffix;
        if removed >= 6 && inserted.saturating_mul(2) < removed {
            concerns.push("proposed replacement removes most of a multi-line span; confirm the complete target scope");
        }
    }
    Ok(
        json!({"path":change.path,"before_sha256":change.before_hash,"after_sha256":change.after_hash,"syntax":syntax,"concerns":concerns,"needs_intent":!concerns.is_empty(),"diff":change.diff(),"scope":"Proposed bytes only; nothing has been applied. Parser acceptance does not establish compiler or behavioral correctness."}),
    )
}

/// A read packet contains unnumbered, exact text and its current revision handle.
/// A clipped read never advertises an editable span.
pub fn read_packet(read: &Value) -> String {
    let path = read["path"].as_str().unwrap_or("");
    match (read["range_handle"].as_str(), read["source"].as_str()) {
        (Some(handle), Some(source)) => format!(
            "Current file {path}, lines {}–{} of {}; handle {handle}:\nThis handle replaces ALL shown lines.\n<source>\n{source}{}\n</source>\n{}",
            read["start_line"],
            read["start_line"].as_u64().unwrap_or(1)
                + source.lines().count().saturating_sub(1) as u64,
            read["total_lines"],
            if source.ends_with('\n') { "" } else { "\n" },
            read["symbols"]
                .as_array()
                .filter(|rows| !rows.is_empty())
                .map(|rows| format!(
                    "Smaller complete symbol targets: {}\n",
                    serde_json::to_string(rows).unwrap_or_default()
                ))
                .unwrap_or_default()
        ),
        _ => format!(
            "File {path}: clipped preview; no edit handle. Read a smaller range before editing.\n{}\n",
            read["text"].as_str().unwrap_or("")
        ),
    }
}

impl Project {
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_scoped_text_edit(
        &self,
        task: &str,
        path: &str,
        handle: &str,
        operation: &str,
        old_text: Option<&str>,
        replacement: &str,
        reason: &str,
    ) -> Result<Change> {
        let (body, start, end) = self.handle_span(task, path, handle)?;
        let span = &body[start..end];
        let new_span = match operation {
            "replace-span" => {
                return self.prepare_text_edit(task, path, handle, replacement, reason);
            }
            "replace-symbol" => {
                let syntax = crate::syntax::chunks(path, &body)?;
                ensure!(
                    syntax.parser.is_some(),
                    "This language has no qualified symbol parser; use a current span or exact-text target"
                );
                ensure!(
                    syntax
                        .chunks
                        .iter()
                        .any(|c| c.start_byte == start && c.end_byte == end),
                    "Use a complete symbol handle from read for replace-symbol, not a line-range handle"
                );
                replacement.to_owned()
            }
            "replace-text" => {
                let old = old_text.filter(|s| !s.is_empty()).ok_or_else(|| {
                    anyhow::anyhow!(
                        "replace-text requires nonempty exact old_text inside the current handle"
                    )
                })?;
                ensure!(
                    span.matches(old).count() == 1,
                    "old_text must occur exactly once inside the selected current span"
                );
                span.replacen(old, replacement, 1)
            }
            "insert-before" => format!("{replacement}{span}"),
            "insert-after" => format!("{span}{replacement}"),
            _ => anyhow::bail!(
                "Choose replace-span, replace-symbol, replace-text, insert-before or insert-after"
            ),
        };
        self.prepare_handle_edit(task, path, handle, &new_span, reason)
    }
    /// Merge useful current source instead of letting a filename mention disable
    /// retrieval. Large archive directories do not require a cold full-text index.
    pub fn compact_candidates(&mut self, task: &str, query: &str) -> Result<Vec<Value>> {
        let paths = self.browse()?;
        let terms = retrieval_terms(query);
        let mut ranked: BTreeMap<String, (usize, usize, BTreeSet<String>)> = BTreeMap::new();
        let mut add = |path: String, score: usize, line: usize, why: &str| {
            let row = ranked.entry(path).or_insert((0, line, BTreeSet::new()));
            if score > row.0 {
                row.0 = score;
                row.1 = line;
            }
            row.2.insert(why.to_owned());
        };
        for term in query
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
                add(
                    path.clone(),
                    40,
                    1,
                    "explicit path (mention does not authorize changing it)",
                );
            }
        }
        if let Some(check) = self.latest_check(task)? {
            for v in crate::workflow::diagnostic_locations(self, &check)? {
                if let (Some(path), Some(line)) = (v["path"].as_str(), v["line"].as_u64()) {
                    add(
                        path.into(),
                        100,
                        line as usize,
                        "diagnostic with verified project root",
                    );
                }
            }
        }
        for change in self
            .changes(Some(task))?
            .into_iter()
            .filter(|c| c.status == "applied")
            .take(8)
        {
            if paths.contains(&change.path) {
                add(change.path, 65, 1, "current task change");
            }
        }
        // Inspect only source files, with cancellation between files. This keeps
        // explicit navigation useful in a repository with large evidence archives.
        for path in paths
            .iter()
            .filter(|p| {
                matches!(
                    std::path::Path::new(p).extension().and_then(|e| e.to_str()),
                    Some("rs" | "py" | "js" | "jsx" | "mjs" | "cjs")
                )
            })
            .take(128)
        {
            self.check_cancel()?;
            let Some(bytes) = self.bytes(path)? else {
                continue;
            };
            let Ok(body) = std::str::from_utf8(&bytes) else {
                continue;
            };
            let syntax = crate::syntax::chunks(path, body)?;
            let mut best = (0, 1);
            for symbol in &syntax.chunks {
                let name = symbol.name.to_lowercase();
                let score = terms
                    .iter()
                    .filter(|t| name == **t || name.split('_').any(|s| s == t.as_str()))
                    .count()
                    * 80;
                if score > best.0 {
                    best = (score, symbol.start_line);
                }
            }
            if best.0 > 0 {
                add(path.clone(), best.0, best.1, "target symbol");
            } else {
                let lower = body.to_lowercase();
                let score = terms
                    .iter()
                    .filter(|t| t.len() > 2 && lower.contains(t.as_str()))
                    .count()
                    * 8;
                if score > 0 {
                    add(path.clone(), score, 1, "source text");
                }
            }
        }
        // Use the full lexical index when focused reads found nothing, or when
        // the project is small. Explicit paths and ranked symbols stay merged.
        if (ranked.is_empty() || paths.len() <= 64) && !query.trim().is_empty() {
            for (i, hit) in self
                .search(&query[..query.floor_char_boundary(1000)])?
                .into_iter()
                .enumerate()
            {
                if let Some(path) = hit["path"].as_str() {
                    let row = ranked.entry(path.into()).or_insert((
                        20usize.saturating_sub(i),
                        hit["start_line"].as_u64().unwrap_or(1) as usize,
                        BTreeSet::new(),
                    ));
                    row.2.insert("lexical match".into());
                }
            }
        }
        if ranked.is_empty() && paths.len() <= 4 {
            for path in &paths {
                ranked.insert(
                    path.clone(),
                    (1, 1, BTreeSet::from(["small project fallback".into()])),
                );
            }
        }
        let selected = ranked
            .iter()
            .filter(|(_, row)| row.0 >= 30)
            .take(6)
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();
        for path in selected {
            let Some(bytes) = self.bytes(&path)? else {
                continue;
            };
            let Ok(body) = std::str::from_utf8(&bytes) else {
                continue;
            };
            for dep in crate::syntax::chunks(&path, body)?
                .dependencies
                .into_iter()
                .take(8)
            {
                let module = dep.module.or_else(|| {
                    dep.statement.strip_prefix("use ").map(|s| {
                        s.trim_end_matches(';')
                            .split("::")
                            .find(|s| !matches!(*s, "crate" | "self" | "super"))
                            .unwrap_or("")
                            .to_owned()
                    })
                });
                if let Some(module) = module {
                    let stem = module.trim_start_matches("./").replace('.', "/");
                    for neighbor in paths
                        .iter()
                        .filter(|p| {
                            **p != path
                                && ["py", "rs", "js", "mjs"]
                                    .iter()
                                    .any(|ext| p.ends_with(&format!("{stem}.{ext}")))
                                || (**p != path && p.ends_with(&format!("{stem}/__init__.py")))
                        })
                        .take(3)
                    {
                        let row =
                            ranked
                                .entry(neighbor.clone())
                                .or_insert((24, 1, BTreeSet::new()));
                        row.2.insert(format!("import neighbor of {path}"));
                    }
                }
            }
        }
        let mut rows = ranked.into_iter().collect::<Vec<_>>();
        rows.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(&b.0)));
        Ok(rows.into_iter().take(12).map(|(path,(score,line,reasons))|json!({"path":path,"start_line":line,"score":score,"reasons":reasons})).collect())
    }
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
        let active_requests = self.active_requests(task)?;
        let pinned = self.pinned()?;
        let mut out = format!("Current check status: {}.\n", state.check_status);
        let continuing = query.trim() != state.goal.trim();
        let original_active = active_requests.iter().any(|r| r.body == state.goal);
        if continuing && original_active {
            out.push_str(&format!(
                "Original task request (context; the latest user message takes precedence): {}\n",
                state.goal
            ));
        }
        for request in &active_requests {
            if request.body.trim() != state.goal.trim() && request.body.trim() != query.trim() {
                out.push_str(&format!("User request #{} (exact text; later explicit corrections take precedence):\n{}\n",request.seq,request.body));
            }
        }
        let relevant_query = std::iter::once(query.to_owned())
            .chain(
                active_requests
                    .iter()
                    .rev()
                    .filter(|r| r.body.trim() != query.trim())
                    .map(|r| r.body.clone()),
            )
            .collect::<Vec<_>>()
            .join("\n");
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
            "Active user requests, pinned requirements and current check status exceed the compact context budget. Inspect What Alt currently understands; explicitly retire superseded requirements, increase context, or split the task. Nothing was silently discarded."
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
        let ranked_candidates = self.compact_candidates(task, &relevant_query)?;
        let retrieval_method = "merged-path-symbol-diagnostic-v2";
        let candidates = ranked_candidates
            .iter()
            .filter_map(|v| {
                Some((
                    v["path"].as_str()?.to_owned(),
                    v["start_line"].as_u64()? as usize,
                ))
            })
            .collect::<Vec<_>>();
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
        view["ranked_candidates"] = json!(ranked_candidates);
        view["active_user_requests"] = json!(active_requests);
        view["request_history"] = json!(self.request_history(task)?);
        self.db.execute(
            "INSERT OR REPLACE INTO context_views VALUES(?1,?2)",
            params![task, view.to_string()],
        )?;
        Ok(out)
    }
}
