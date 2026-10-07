//! Reviewed immutable offline instruction candidates. No hosted optimizer is implicit.
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
fn path(root: &Path, id: &str) -> Result<PathBuf> {
    ensure!(
        id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit()),
        "Instruction id must be its SHA256"
    );
    Ok(root.join("instructions").join(id))
}
pub fn text(root: &Path, id: &str) -> Result<String> {
    let body = std::fs::read_to_string(path(root, id)?.join("procedure.md"))?;
    ensure!(
        crate::project::digest(body.as_bytes()) == id,
        "Instruction body changed after registration"
    );
    Ok(body)
}
pub fn metadata(root: &Path, id: &str) -> Result<Value> {
    let directory = path(root, id)?;
    let bytes = std::fs::read(directory.join("metadata.json"))?;
    let registration: Value =
        serde_json::from_slice(&std::fs::read(directory.join("registration.json"))?)?;
    ensure!(
        registration["instruction_sha256"].as_str() == Some(id)
            && registration["metadata_sha256"] == crate::project::digest(&bytes),
        "Instruction provenance changed after registration"
    );
    Ok(serde_json::from_slice(&bytes)?)
}
pub fn register(root: &Path, body: &str, meta: &Value) -> Result<Value> {
    ensure!(
        !body.trim().is_empty() && body.len() <= 5000,
        "Use a nonempty procedure of at most 5000 UTF-8 bytes"
    );
    ensure!(
        meta["schema"] == 1 && meta["target"] == "operator-supplement",
        "This registry accepts version 1 operator supplements; it preserves the core tool contract"
    );
    let optimizer = &meta["optimizer"];
    ensure!(
        optimizer["kind"] == "human"
            || (optimizer["kind"] == "model"
                && optimizer["uncensored"] == true
                && optimizer["repository"]
                    .as_str()
                    .is_some_and(|s| !s.is_empty())
                && optimizer["artifact_sha256"]
                    .as_str()
                    .is_some_and(|s| s.len() == 64)
                && optimizer["revision"]
                    .as_str()
                    .is_some_and(|s| s.len() == 40 || s.len() == 64)),
        "Identify the explicitly permitted optimizer or human author"
    );
    let development = meta["development_families"]
        .as_array()
        .context("Development families missing")?;
    let validation = meta["validation_families"]
        .as_array()
        .context("Separate validation families missing")?;
    ensure!(
        !development.is_empty()
            && !validation.is_empty()
            && !development.iter().any(|g| validation.contains(g)),
        "Development and validation families must be nonempty and disjoint"
    );
    ensure!(
        development.iter().chain(validation).all(|g| g
            .as_str()
            .is_some_and(|s| !s.starts_with("sealed-") && !s.is_empty())),
        "Sealed final-test families cannot select instructions"
    );
    let id = crate::project::digest(body.as_bytes());
    let directory = path(root, &id)?;
    if directory.exists() {
        ensure!(
            text(root, &id)? == body && metadata(root, &id)? == *meta,
            "Immutable instruction candidate already has different provenance"
        );
    } else {
        crate::config::private_dir(&directory)?;
        crate::config::atomic_write(&directory.join("procedure.md"), body.as_bytes())?;
        crate::config::atomic_write(
            &directory.join("metadata.json"),
            &serde_json::to_vec_pretty(meta)?,
        )?;
        crate::config::atomic_write(
            &directory.join("registration.json"),
            &serde_json::to_vec_pretty(
                &json!({"schema":1,"instruction_sha256":id,"metadata_sha256":crate::capability::file_sha256(&directory.join("metadata.json"))?}),
            )?,
        )?;
    }
    Ok(json!({"id":id,"registered":true,"promoted":false,"metadata":meta}))
}
pub fn validation(root: &Path, id: &str) -> Result<()> {
    let mut meta = metadata(root, id)?;
    if let Ok(bytes) = std::fs::read(path(root, id)?.join("review.json")) {
        let review: Value = serde_json::from_slice(&bytes)?;
        ensure!(
            review["instruction_sha256"].as_str() == Some(id),
            "Review belongs to another instruction"
        );
        for field in ["review", "validation_evidence"] {
            meta[field] = review[field].clone();
        }
    }
    validate_data(root, id, &meta)
}
fn validate_data(root: &Path, id: &str, meta: &Value) -> Result<()> {
    ensure!(
        meta["review"]["status"] == "approved"
            && meta["review"]["reviewer"]
                .as_str()
                .is_some_and(|s| !s.trim().is_empty()),
        "Review the candidate and its independent validation before promotion"
    );
    let evidence = meta["validation_evidence"]
        .as_array()
        .context("Validation evidence missing")?;
    ensure!(!evidence.is_empty(), "Validation evidence empty");
    let allowed = meta["validation_families"]
        .as_array()
        .context("Validation split missing")?;
    let mut measured = std::collections::BTreeSet::new();
    for reference in evidence {
        let file = Path::new(
            reference["path"]
                .as_str()
                .context("Evidence path missing")?,
        );
        ensure!(
            crate::capability::file_sha256(file)? == reference["sha256"].as_str().unwrap_or(""),
            "Validation evidence changed"
        );
        let row: Value = serde_json::from_slice(&std::fs::read(file)?)?;
        ensure!(
            row["passed"] == true
                && row["oracle_unchanged"] == true
                && row["after"]["completion_observed"] == true
                && row["after"]["passed"] == true
                && row["after"]["exit"].as_i64() == Some(0)
                && row["before"]["passed"] == false,
            "Validation did not pass independent behavioral checks"
        );
        let attempt = file
            .parent()
            .context("Validation attempt directory missing")?;
        let campaign: Value = serde_json::from_slice(&std::fs::read(
            attempt
                .parent()
                .context("Validation campaign missing")?
                .join("configuration.json"),
        )?)?;
        ensure!(
            campaign["oracle_version"] == 5
                && campaign["partition"] == "development"
                && campaign["instruction_sha256"].as_str() == Some(id),
            "Use v5 validation runs that actually trial this exact supplement, outside the sealed final test"
        );
        let manifest: std::collections::BTreeMap<String, String> =
            serde_json::from_slice(&std::fs::read(attempt.join("evidence-sha256.json"))?)?;
        ensure!(
            manifest.contains_key("report.json")
                && manifest.keys().any(|s| s.ends_with("-request.json")),
            "Validation lacks captured inference evidence"
        );
        for (name, digest) in &manifest {
            ensure!(
                !Path::new(name).is_absolute()
                    && !Path::new(name)
                        .components()
                        .any(|c| matches!(c, std::path::Component::ParentDir)),
                "Evidence path escapes attempt"
            );
            ensure!(
                crate::capability::file_sha256(&attempt.join(name))? == *digest,
                "Validation raw evidence changed"
            );
        }
        let marker = format!("Experimental operator supplement {id}");
        ensure!(
            manifest
                .keys()
                .filter(|s| s.ends_with("-request.json"))
                .any(|s| std::fs::read_to_string(attempt.join(s))
                    .is_ok_and(|body| body.contains(&marker))),
            "Captured request did not contain the candidate being reviewed"
        );
        let sources = row["resulting_source"]
            .as_object()
            .context("Validation source snapshot missing")?;
        for (name, body) in sources {
            ensure!(
                !Path::new(name).is_absolute()
                    && !Path::new(name)
                        .components()
                        .any(|c| matches!(c, std::path::Component::ParentDir)),
                "Validation source path escape"
            );
            ensure!(
                Some(std::fs::read_to_string(attempt.join("project").join(name))?.as_str())
                    == body.as_str(),
                "Validation source changed after independent checks"
            );
        }
        let family = row["case"].as_str().context("Validation family missing")?;
        ensure!(
            allowed.contains(&json!(family)) && !family.starts_with("sealed-"),
            "Validation evidence belongs to another split"
        );
        measured.insert(family.to_owned());
    }
    ensure!(
        allowed
            .iter()
            .all(|f| f.as_str().is_some_and(|f| measured.contains(f))),
        "Validation families have missing attempts"
    );
    ensure!(
        meta["cost"]["requests"].as_u64().is_some_and(|n| n > 0)
            && meta["cost"]["wall_seconds"]
                .as_f64()
                .is_some_and(|n| n > 0.0),
        "Record the full offline optimization request/time cost"
    );
    text(root, id)?;
    Ok(())
}
pub fn activate(
    root: &Path,
    prefs: &mut crate::config::Preferences,
    id: Option<&str>,
) -> Result<Value> {
    if let Some(id) = id {
        validation(root, id)?;
    }
    let before = prefs.instruction_version.clone();
    prefs.instruction_version = id.map(str::to_owned);
    prefs.save(root)?;
    let event = json!({"previous":before,"active":id,"explicit_user_promotion":true,"scope":"Reviewed instructions; performance gain requires matched complete validation","id":uuid::Uuid::new_v4().to_string()});
    crate::config::atomic_write(
        &root
            .join("instruction-history")
            .join(format!("{}.json", event["id"].as_str().unwrap())),
        &serde_json::to_vec_pretty(&event)?,
    )?;
    Ok(event)
}
pub fn list(root: &Path) -> Result<Vec<Value>> {
    let directory = root.join("instructions");
    if !directory.exists() {
        return Ok(vec![]);
    }
    let mut rows = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        if entry.path().is_dir() {
            let id = entry.file_name().to_string_lossy().into_owned();
            rows.push(json!({"id":id,"metadata":metadata(root,&id)?}));
        }
    }
    Ok(rows)
}

pub fn review(root: &Path, id: &str, receipt: &Value) -> Result<Value> {
    text(root, id)?;
    ensure!(
        receipt["instruction_sha256"].as_str() == Some(id)
            && receipt["review"]["status"] == "approved",
        "Approve the exact reviewed instruction body"
    );
    let mut meta = metadata(root, id)?;
    for field in ["review", "validation_evidence"] {
        meta[field] = receipt[field].clone();
    }
    validate_data(root, id, &meta)?;
    let file = path(root, id)?.join("review.json");
    if file.exists() {
        ensure!(
            serde_json::from_slice::<Value>(&std::fs::read(&file)?)? == *receipt,
            "Review receipt is immutable; keep its evidence"
        );
    } else {
        crate::config::atomic_write(&file, &serde_json::to_vec_pretty(receipt)?)?;
    }
    validation(root, id)?;
    Ok(json!({"id":id,"reviewed":true,"promoted":false}))
}
