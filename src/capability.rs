//! Configuration identity. Claims without exact weights/template identity stay unqualified.
use anyhow::Result;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path};

pub fn file_sha256(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
pub fn identity(
    root: &Path,
    preferences: &crate::config::Preferences,
    profile: &crate::config::Profile,
    template: Option<&str>,
) -> Result<Value> {
    let artifact = profile
        .local_model
        .as_ref()
        .map(|id| crate::models::artifact(root, id))
        .transpose()?;
    let runtime_path = profile
        .local_model
        .as_ref()
        .and_then(|_| crate::runtime::find_runtime(root, preferences));
    let runtime = runtime_path
        .as_ref()
        .filter(|p| p.is_file())
        .map(|p| file_sha256(p))
        .transpose()?;
    let mut libraries = std::collections::BTreeMap::new();
    if let Some(parent) = runtime_path.as_ref().and_then(|p| p.parent()) {
        for entry in std::fs::read_dir(parent)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if (name.ends_with(".so") || name.contains(".so.")) && entry.path().is_file() {
                libraries.insert(name, file_sha256(&entry.path())?);
            }
        }
    }
    let engine_path = profile
        .local_model
        .as_ref()
        .and_then(|_| crate::runtime::find_engine(root, Path::new("goose"), preferences));
    let engine = engine_path
        .as_ref()
        .filter(|p| p.is_file())
        .map(|p| file_sha256(p))
        .transpose()?;
    let build: Value = serde_json::from_str(crate::BUILD_INFO)?;
    let mut value = json!({"schema":1,"source_sha256":build["source_sha256"],"model_id":profile.model,"provider":profile.provider,"endpoint":if profile.local_model.is_some(){"managed"}else{&profile.endpoint},"artifact_sha256":artifact.as_ref().map(|a|&a.sha256),"runtime_sha256":runtime,"runtime_libraries":libraries,"engine_sha256":engine,"context":profile.context_tokens,"max_turns":profile.max_turns,"runtime_settings":preferences.runtime,"tool_profile":preferences.tool_profile,"access":preferences.access_policy,"tool_schema_sha256":crate::project::digest(&serde_json::to_vec(&crate::toolbox::focused_tools(preferences.tool_profile))?),"operator_sha256":crate::project::digest(include_bytes!("../prompts/operator.md")),"template_sha256":template.map(|t|crate::project::digest(t.as_bytes())),"identity_complete":artifact.is_some()&&runtime.is_some()&&engine.is_some()&&template.is_some()});
    value["effective_inference"] = profile.effective_inference().accounting(profile);
    value["workflow"] = json!(preferences.workflow);
    value["instruction_version"] = json!(preferences.instruction_version);
    value["active_skill"] = preferences
        .active_skill
        .as_ref()
        .map(|id| crate::skills::get(id))
        .transpose()?
        .map(|s| json!({"id":s.id,"version":s.version,"sha256":s.sha256}))
        .unwrap_or(Value::Null);
    value["fingerprint"] = json!(crate::project::digest(&serde_json::to_vec(&value)?));
    Ok(value)
}
