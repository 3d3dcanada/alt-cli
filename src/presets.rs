//! A03 provider presets: endpoint, key-variable name and thinking dialect.
//! A preset only fills fields; keys stay environment-variable references.
use crate::{config::Provider, inference::Dialect};

#[derive(Debug, Clone, Copy)]
pub struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    pub provider: Provider,
    pub endpoint: &'static str,
    /// The environment variable that holds the key; never the key itself.
    pub key_env: Option<&'static str>,
    pub dialect: Dialect,
    pub hosted: bool,
    pub note: &'static str,
}

pub const PRESETS: [Preset; 13] = [
    Preset {
        id: "nvidia",
        name: "NVIDIA NIM",
        provider: Provider::Openai,
        endpoint: "https://integrate.api.nvidia.com/v1",
        key_env: Some("NVIDIA_API_KEY"),
        dialect: Dialect::ChatTemplate,
        hosted: true,
        note: "Hosted catalog; key from build.nvidia.com",
    },
    Preset {
        id: "openrouter",
        name: "OpenRouter",
        provider: Provider::Openai,
        endpoint: "https://openrouter.ai/api/v1",
        key_env: Some("OPENROUTER_API_KEY"),
        dialect: Dialect::OpenaiEffort,
        hosted: true,
        note: "Many providers behind one key",
    },
    Preset {
        id: "groq",
        name: "Groq",
        provider: Provider::Openai,
        endpoint: "https://api.groq.com/openai/v1",
        key_env: Some("GROQ_API_KEY"),
        dialect: Dialect::OpenaiEffort,
        hosted: true,
        note: "Fast hosted open models",
    },
    Preset {
        id: "together",
        name: "Together AI",
        provider: Provider::Openai,
        endpoint: "https://api.together.xyz/v1",
        key_env: Some("TOGETHER_API_KEY"),
        dialect: Dialect::OpenaiEffort,
        hosted: true,
        note: "Hosted open models",
    },
    Preset {
        id: "deepseek",
        name: "DeepSeek",
        provider: Provider::Openai,
        endpoint: "https://api.deepseek.com/v1",
        key_env: Some("DEEPSEEK_API_KEY"),
        dialect: Dialect::OpenaiEffort,
        hosted: true,
        note: "DeepSeek's own API",
    },
    Preset {
        id: "mistral",
        name: "Mistral",
        provider: Provider::Openai,
        endpoint: "https://api.mistral.ai/v1",
        key_env: Some("MISTRAL_API_KEY"),
        dialect: Dialect::OpenaiEffort,
        hosted: true,
        note: "Mistral's own API",
    },
    Preset {
        id: "gemini",
        name: "Gemini (OpenAI-compatible)",
        provider: Provider::Openai,
        endpoint: "https://generativelanguage.googleapis.com/v1beta/openai",
        key_env: Some("GEMINI_API_KEY"),
        dialect: Dialect::OpenaiEffort,
        hosted: true,
        note: "Google's OpenAI-compatible endpoint",
    },
    Preset {
        id: "cerebras",
        name: "Cerebras",
        provider: Provider::Openai,
        endpoint: "https://api.cerebras.ai/v1",
        key_env: Some("CEREBRAS_API_KEY"),
        dialect: Dialect::OpenaiEffort,
        hosted: true,
        note: "Fast hosted open models",
    },
    Preset {
        id: "lmstudio",
        name: "LM Studio",
        provider: Provider::Openai,
        endpoint: "http://127.0.0.1:1234/v1",
        key_env: None,
        dialect: Dialect::Auto,
        hosted: false,
        note: "LM Studio's local server",
    },
    Preset {
        id: "vllm",
        name: "vLLM / SGLang",
        provider: Provider::Openai,
        endpoint: "http://127.0.0.1:8000/v1",
        key_env: None,
        dialect: Dialect::ChatTemplate,
        hosted: false,
        note: "Self-hosted vLLM or SGLang",
    },
    Preset {
        id: "llamacpp",
        name: "llama.cpp server",
        provider: Provider::Openai,
        endpoint: "http://127.0.0.1:8080/v1",
        key_env: None,
        dialect: Dialect::LlamaCpp,
        hosted: false,
        note: "An external llama-server you run",
    },
    Preset {
        id: "ollama",
        name: "Ollama",
        provider: Provider::Ollama,
        endpoint: "http://127.0.0.1:11434",
        key_env: None,
        dialect: Dialect::Auto,
        hosted: false,
        note: "A model already installed in Ollama",
    },
    Preset {
        id: "custom",
        name: "Custom",
        provider: Provider::Openai,
        endpoint: "http://127.0.0.1:8080/v1",
        key_env: None,
        dialect: Dialect::Auto,
        hosted: false,
        note: "Any OpenAI-compatible address",
    },
];

pub fn find(id: &str) -> anyhow::Result<&'static Preset> {
    let id = id.trim().to_ascii_lowercase();
    PRESETS
        .iter()
        .find(|p| p.id == id || p.name.eq_ignore_ascii_case(&id))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown preset `{id}`. Choose one of: {}",
                PRESETS.iter().map(|p| p.id).collect::<Vec<_>>().join(", ")
            )
        })
}

/// Case-insensitive search over model ids for long hosted catalogs.
pub fn filter_models<'a>(
    models: &'a [crate::models::ServerModel],
    query: &str,
) -> Vec<&'a crate::models::ServerModel> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    models
        .iter()
        .filter(|m| {
            let id = m.id.to_lowercase();
            words.iter().all(|w| id.contains(w))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presets_are_unique_and_hosted_ones_name_a_key_variable() {
        let mut ids = std::collections::BTreeSet::new();
        for p in PRESETS {
            assert!(ids.insert(p.id), "duplicate {}", p.id);
            assert!(reqwest::Url::parse(p.endpoint).is_ok());
            if p.hosted {
                assert!(p.endpoint.starts_with("https://"), "{}", p.id);
                let key = p.key_env.unwrap();
                assert!(key.chars().all(|c| c.is_ascii_uppercase() || c == '_'));
            }
        }
        assert_eq!(find("NVIDIA").unwrap().key_env, Some("NVIDIA_API_KEY"));
        assert!(find("nope").is_err());
    }
}
