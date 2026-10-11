//! A02 open parameter surface: one table of named request and runtime
//! parameters shared by `alt params` and the TUI's Advanced parameters.
use crate::{
    config::{Config, Preferences},
    inference::{OllamaContext, Settings},
};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Saved on the selected connection; sent with each model request.
    Request,
    /// Saved in preferences; used when Alt starts its own llama-server.
    Runtime,
}

pub struct Param {
    pub key: &'static str,
    pub label: &'static str,
    pub scope: Scope,
    pub hint: &'static str,
}

pub const PARAMS: [Param; 23] = [
    Param {
        key: "repeat_penalty",
        label: "Repeat penalty",
        scope: Scope::Request,
        hint: "1 is off; 1.05–1.15 discourages loops. llama.cpp, Ollama, vLLM",
    },
    Param {
        key: "presence_penalty",
        label: "Presence penalty",
        scope: Scope::Request,
        hint: "-2 to 2; positive values encourage new topics",
    },
    Param {
        key: "frequency_penalty",
        label: "Frequency penalty",
        scope: Scope::Request,
        hint: "-2 to 2; positive values discourage repetition",
    },
    Param {
        key: "seed",
        label: "Seed",
        scope: Scope::Request,
        hint: "Whole number for repeatable sampling where the server supports it",
    },
    Param {
        key: "stop",
        label: "Stop sequences",
        scope: Scope::Request,
        hint: "Separate with |, or a JSON array",
    },
    Param {
        key: "typical_p",
        label: "Typical-p",
        scope: Scope::Request,
        hint: ">0 and <=1. llama.cpp and Ollama",
    },
    Param {
        key: "extra_body",
        label: "Extra request JSON",
        scope: Scope::Request,
        hint: "JSON object merged last; model, messages, tools, stream and output fields are refused",
    },
    Param {
        key: "ollama_context",
        label: "Ollama context",
        scope: Scope::Request,
        hint: "profile sends options.num_ctx from the connection; server-default sends nothing",
    },
    Param {
        key: "keep_alive",
        label: "Ollama keep-alive",
        scope: Scope::Request,
        hint: "How long Ollama keeps the model loaded: 5m, 1h, 0 or -1",
    },
    Param {
        key: "mmproj",
        label: "Vision projector (mmproj)",
        scope: Scope::Runtime,
        hint: "Path to the model's mmproj GGUF",
    },
    Param {
        key: "n_cpu_moe",
        label: "MoE layers on CPU",
        scope: Scope::Runtime,
        hint: "Keep this many MoE expert layers on the CPU (--n-cpu-moe)",
    },
    Param {
        key: "override_tensor",
        label: "Tensor buffer override",
        scope: Scope::Runtime,
        hint: "pattern=buffer list for --override-tensor, such as exps=CPU",
    },
    Param {
        key: "tensor_split",
        label: "Tensor split",
        scope: Scope::Runtime,
        hint: "Proportions per GPU, such as 3,1",
    },
    Param {
        key: "main_gpu",
        label: "Main GPU",
        scope: Scope::Runtime,
        hint: "GPU index for scratch and small tensors",
    },
    Param {
        key: "rope_scaling",
        label: "RoPE scaling",
        scope: Scope::Runtime,
        hint: "none, linear or yarn",
    },
    Param {
        key: "rope_scale",
        label: "RoPE scale factor",
        scope: Scope::Runtime,
        hint: "Context scaling factor, such as 2",
    },
    Param {
        key: "yarn_orig_ctx",
        label: "YaRN original context",
        scope: Scope::Runtime,
        hint: "The model's trained context, such as 32768",
    },
    Param {
        key: "mlock",
        label: "Lock model in RAM (mlock)",
        scope: Scope::Runtime,
        hint: "true or false; prevents swapping out",
    },
    Param {
        key: "no_mmap",
        label: "Disable memory mapping",
        scope: Scope::Runtime,
        hint: "true or false; loads the whole file into RAM",
    },
    Param {
        key: "chat_template_file",
        label: "Chat template file",
        scope: Scope::Runtime,
        hint: "Path to a Jinja template that replaces the model's",
    },
    Param {
        key: "draft_model",
        label: "Speculative draft model",
        scope: Scope::Runtime,
        hint: "Path to a small GGUF from the same family",
    },
    Param {
        key: "draft_max",
        label: "Draft tokens",
        scope: Scope::Runtime,
        hint: "1–64 tokens drafted per step",
    },
    Param {
        key: "extra_args",
        label: "Extra runtime arguments",
        scope: Scope::Runtime,
        hint: "Space-separated or JSON array; --host, --port, --api-key, --alias, --model, --ctx-size are refused",
    },
];

pub fn find(key: &str) -> Result<&'static Param> {
    PARAMS.iter().find(|p| p.key == key).with_context(|| {
        format!(
            "Unknown parameter `{key}`. Known: {}",
            PARAMS.iter().map(|p| p.key).collect::<Vec<_>>().join(", ")
        )
    })
}

fn number<T: std::str::FromStr>(key: &str, value: &str) -> Result<T> {
    value
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("`{key}` needs a number"))
}

fn list(value: &str, separator: char) -> Result<Vec<String>> {
    let value = value.trim();
    if value.starts_with('[') {
        return serde_json::from_str(value).context("Use a JSON array of strings");
    }
    Ok(value
        .split(separator)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect())
}

/// Set (`Some`) or unset (`None`) one request parameter on these settings.
pub fn set_request(settings: &mut Settings, key: &str, value: Option<&str>) -> Result<()> {
    match (key, value) {
        ("repeat_penalty", v) => settings.repeat_penalty = v.map(|v| number(key, v)).transpose()?,
        ("presence_penalty", v) => {
            settings.presence_penalty = v.map(|v| number(key, v)).transpose()?
        }
        ("frequency_penalty", v) => {
            settings.frequency_penalty = v.map(|v| number(key, v)).transpose()?
        }
        ("seed", v) => settings.seed = v.map(|v| number(key, v)).transpose()?,
        ("typical_p", v) => settings.typical_p = v.map(|v| number(key, v)).transpose()?,
        ("stop", v) => settings.stop = v.map(|v| list(v, '|')).transpose()?.unwrap_or_default(),
        ("extra_body", v) => {
            settings.extra_body = v.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
        }
        ("keep_alive", v) => {
            settings.keep_alive = v.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
        }
        ("ollama_context", None) => settings.ollama_context = OllamaContext::Profile,
        ("ollama_context", Some(v)) => {
            settings.ollama_context = match v.trim() {
                "profile" => OllamaContext::Profile,
                "server-default" | "server" | "default" => OllamaContext::ServerDefault,
                _ => bail!("Ollama context is `profile` or `server-default`"),
            }
        }
        _ => bail!("`{key}` is not a request parameter"),
    }
    Ok(())
}

fn path(value: &str) -> Result<PathBuf> {
    let path = PathBuf::from(value.trim());
    anyhow::ensure!(path.is_file(), "{} is not a readable file", path.display());
    Ok(path)
}

/// Set (`Some`) or unset (`None`) one owned-runtime parameter.
pub fn set_runtime(
    settings: &mut crate::runtime::Settings,
    key: &str,
    value: Option<&str>,
) -> Result<()> {
    let text = |v: Option<&str>| v.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty());
    let flag = |v: Option<&str>| -> Result<bool> {
        Ok(match v.map(str::trim) {
            None | Some("false") | Some("off") => false,
            Some("true") | Some("on") => true,
            Some(_) => bail!("`{key}` is true or false"),
        })
    };
    match key {
        "mmproj" => settings.mmproj = value.map(path).transpose()?,
        "chat_template_file" => settings.chat_template_file = value.map(path).transpose()?,
        "draft_model" => settings.draft_model = value.map(path).transpose()?,
        "n_cpu_moe" => settings.n_cpu_moe = value.map(|v| number(key, v)).transpose()?,
        "main_gpu" => settings.main_gpu = value.map(|v| number(key, v)).transpose()?,
        "rope_scale" => settings.rope_scale = value.map(|v| number(key, v)).transpose()?,
        "yarn_orig_ctx" => settings.yarn_orig_ctx = value.map(|v| number(key, v)).transpose()?,
        "draft_max" => settings.draft_max = value.map(|v| number(key, v)).transpose()?,
        "override_tensor" => settings.override_tensor = text(value),
        "tensor_split" => settings.tensor_split = text(value),
        "rope_scaling" => settings.rope_scaling = text(value),
        "mlock" => settings.mlock = flag(value)?,
        "no_mmap" => settings.no_mmap = flag(value)?,
        "extra_args" => {
            settings.extra_args = value.map(|v| list(v, ' ')).transpose()?.unwrap_or_default()
        }
        _ => bail!("`{key}` is not a runtime parameter"),
    }
    Ok(())
}

/// Validate and save one change. Request parameters belong to `profile`;
/// runtime parameters to preferences. Returns the new effective view.
pub fn change(root: &Path, profile: Option<&str>, key: &str, value: Option<&str>) -> Result<Value> {
    let param = find(key)?;
    match param.scope {
        Scope::Request => {
            let mut config = Config::read(root)?;
            let name = config.profile(profile)?.0.to_owned();
            let entry = config
                .profiles
                .get_mut(&name)
                .context("Connection disappeared")?;
            let mut settings = entry.effective_inference();
            set_request(&mut settings, key, value)?;
            settings.validate(entry)?;
            entry.inference = Some(settings);
            config.save(root)?;
        }
        Scope::Runtime => {
            let mut preferences = Preferences::load(root)?;
            set_runtime(&mut preferences.runtime, key, value)?;
            preferences.runtime.validate()?;
            preferences.save(root)?;
        }
    }
    show(root, profile)
}

fn current(settings: &Settings, runtime: &crate::runtime::Settings, key: &str) -> Value {
    let request = serde_json::to_value(settings).unwrap_or_default();
    let owned = serde_json::to_value(runtime).unwrap_or_default();
    match find(key).map(|p| p.scope) {
        Ok(Scope::Request) => request.get(key).cloned().unwrap_or(Value::Null),
        _ => owned.get(key).cloned().unwrap_or(Value::Null),
    }
}

/// Every parameter with its current value, plus the exact effective request
/// fields and the owned-runtime argv Alt would use.
pub fn show(root: &Path, profile: Option<&str>) -> Result<Value> {
    let config = Config::load(root)?;
    let preferences = Preferences::load(root)?;
    let selected = config.profile(profile).ok();
    let settings = selected
        .map(|(_, p)| p.effective_inference())
        .unwrap_or_else(|| Settings::legacy(preferences.context_tokens));
    let rows: Vec<Value> = PARAMS
        .iter()
        .map(|p| json!({"key":p.key,"label":p.label,"scope":match p.scope {Scope::Request=>"request",Scope::Runtime=>"runtime"},"value":current(&settings,&preferences.runtime,p.key),"hint":p.hint}))
        .collect();
    let mut body = json!({"model": selected.map(|(_, p)| p.model.clone()).unwrap_or_default(), "messages": [], "stream": true});
    let request = match selected {
        Some((_, p)) => match p.effective_inference().apply(p, &mut body) {
            Ok(()) => {
                if let Some(object) = body.as_object_mut() {
                    object.remove("messages");
                }
                body
            }
            Err(error) => json!({"error": error.to_string()}),
        },
        None => json!({"note": "No connection selected"}),
    };
    let argv = selected.map(|(_, p)| {
        crate::runtime::server_args(
            &preferences.runtime,
            p,
            Path::new("<model.gguf>"),
            0,
            preferences.runtime.threads,
            Path::new("<private key file>"),
        )
        .into_iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
    });
    Ok(
        json!({"connection": selected.map(|(n, _)| n), "parameters": rows, "effective_request_fields": request, "owned_runtime_argv": argv}),
    )
}

/// One-line display of a current value for menus.
pub fn display(value: &Value) -> String {
    match value {
        Value::Null => "default".into(),
        Value::Array(items) if items.is_empty() => "default".into(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
