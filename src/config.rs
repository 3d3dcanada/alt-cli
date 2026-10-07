use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub default_profile: String,
    pub profiles: BTreeMap<String, Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub provider: Provider,
    /// API root including /v1 for OpenAI-compatible servers; server root for Ollama.
    pub endpoint: String,
    pub model: String,
    pub context_tokens: u32,
    pub max_turns: u32,
    /// A claim supplied by the user/publisher, not a capability assessment.
    #[serde(default)]
    pub uncensored: bool,
    pub api_key_env: Option<String>,
    /// A verified/imported artifact identifier in the local model library.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_model: Option<String>,
    /// Missing older settings are materialized on the next save, preserving their allocation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference: Option<crate::inference::Settings>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    Openai,
    Ollama,
}

impl Profile {
    pub fn validate(&self) -> Result<()> {
        let url = reqwest::Url::parse(&self.endpoint).context("Invalid endpoint URL")?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            bail!("Endpoint must be an HTTP(S) URL without credentials, query, or fragment");
        }
        if self.model.trim().is_empty() {
            bail!("A model name is required");
        }
        if self.context_tokens < 2048 || self.max_turns == 0 || self.max_turns > 100 {
            bail!("Use at least 2048 context tokens and between 1 and 100 turns");
        }
        self.effective_inference().validate(self)?;
        Ok(())
    }
    pub fn effective_inference(&self) -> crate::inference::Settings {
        self.inference
            .clone()
            .unwrap_or_else(|| crate::inference::Settings::legacy(self.context_tokens))
    }

    pub fn models_url(&self) -> String {
        format!(
            "{}/{}",
            self.endpoint.trim_end_matches('/'),
            match self.provider {
                Provider::Openai => "models",
                Provider::Ollama => "api/tags",
            }
        )
    }

    pub fn key(&self) -> Result<Option<String>> {
        self.api_key_env
            .as_ref()
            .map(|name| {
                std::env::var(name).with_context(|| format!("API key variable {name} is not set"))
            })
            .transpose()
    }
}

impl Config {
    pub fn load(root: &Path) -> Result<Self> {
        if !root.join("config.toml").exists() {
            return Ok(Self::default());
        }
        Self::read(root)
    }

    pub fn save(&self, root: &Path) -> Result<()> {
        for (name, profile) in &self.profiles {
            if name.trim().is_empty() || name.chars().any(char::is_control) {
                bail!("Give the connection a readable name");
            }
            profile.validate()?;
        }
        if !self.profiles.is_empty() && !self.profiles.contains_key(&self.default_profile) {
            bail!("Choose an existing connection as the default");
        }
        atomic_write(
            &root.join("config.toml"),
            toml::to_string_pretty(self)?.as_bytes(),
        )
    }

    pub fn upsert(&mut self, name: String, profile: Profile) -> Result<()> {
        profile.validate()?;
        if name.trim().is_empty() {
            bail!("Give this connection a name");
        }
        self.profiles.insert(name.clone(), profile);
        if self.default_profile.is_empty() {
            self.default_profile = name;
        }
        Ok(())
    }
    pub fn read(root: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(root.join("config.toml")).context(
            "No Alt configuration. Open `alt` for guided setup, or run `alt init --model MODEL`",
        )?;
        let mut config: Self = toml::from_str(&text).context("Invalid Alt configuration")?;
        for profile in config.profiles.values_mut() {
            if profile.inference.is_none() {
                profile.inference = Some(profile.effective_inference());
            }
            profile.validate()?;
        }
        if !config.profiles.is_empty() && !config.profiles.contains_key(&config.default_profile) {
            bail!("The default connection is missing from the configuration");
        }
        Ok(config)
    }

    pub fn profile(&self, name: Option<&str>) -> Result<(&str, &Profile)> {
        self.profiles
            .get_key_value(name.unwrap_or(&self.default_profile))
            .map(|(name, profile)| (name.as_str(), profile))
            .context("Unknown profile; check config.toml")
    }

    pub fn create(root: &Path, profile: Profile) -> Result<()> {
        use std::io::Write;
        profile.validate()?;
        private_dir(root)?;
        let config = Self {
            default_profile: "local".into(),
            profiles: BTreeMap::from([("local".into(), profile)]),
        };
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join("config.toml"))
            .context("Configuration already exists or cannot be created")?;
        file.write_all(toml::to_string_pretty(&config)?.as_bytes())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub access_policy: crate::project::Policy,
    pub project: PathBuf,
    pub recent_projects: Vec<PathBuf>,
    pub context_tokens: u32,
    pub max_turns: u32,
    pub mouse: bool,
    pub engine_path: Option<PathBuf>,
    pub runtime_path: Option<PathBuf>,
    pub tool_profile: crate::toolbox::ToolProfile,
    pub runtime: crate::runtime::Settings,
    pub workflow: crate::workflow::Mode,
    pub active_skill: Option<String>,
    pub instruction_version: Option<String>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            access_policy: crate::project::Policy::default(),
            project: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            recent_projects: Vec::new(),
            context_tokens: 8192,
            max_turns: 12,
            mouse: true,
            engine_path: None,
            runtime_path: None,
            runtime: crate::runtime::Settings::default(),
            tool_profile: crate::toolbox::ToolProfile::default(),
            workflow: crate::workflow::Mode::default(),
            active_skill: None,
            instruction_version: None,
        }
    }
}

impl Preferences {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join("preferences.toml");
        let mut preferences: Self = if path.exists() {
            toml::from_str(&std::fs::read_to_string(&path)?)?
        } else {
            Self::default()
        };
        let inline_runtime = if path.exists() {
            toml::from_str::<toml::Value>(&std::fs::read_to_string(&path)?)?
                .get("runtime")
                .is_some()
        } else {
            false
        };
        if !inline_runtime && root.join("runtime.toml").exists() {
            preferences.runtime = (|| -> Result<crate::runtime::Settings> {
                let settings: crate::runtime::Settings =
                    toml::from_str(&std::fs::read_to_string(root.join("runtime.toml"))?)?;
                settings.validate()?;
                Ok(settings)
            })()
            .context("Could not load runtime.toml")?;
        }
        if !(2048..=1_048_576).contains(&preferences.context_tokens)
            || !(1..=100).contains(&preferences.max_turns)
        {
            bail!("Settings need 2,048–1,048,576 context tokens and 1–100 steps");
        }
        preferences.runtime.validate()?;
        Ok(preferences)
    }
    pub fn save(&self, root: &Path) -> Result<()> {
        self.runtime.validate()?;
        atomic_write(
            &root.join("preferences.toml"),
            toml::to_string_pretty(self)?.as_bytes(),
        )
    }
    pub fn choose_project(&mut self, path: &Path) -> Result<()> {
        let path = path
            .canonicalize()
            .context("That folder does not exist or cannot be opened")?;
        if !path.is_dir() {
            bail!("Choose a folder, not a file");
        }
        self.recent_projects.retain(|p| p != &path);
        self.recent_projects.insert(0, path.clone());
        self.recent_projects.truncate(12);
        self.project = path;
        Ok(())
    }
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let parent = path.parent().context("File has no parent directory")?;
    private_dir(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| e.error)?;
    std::fs::File::open(parent)?.sync_all()?;
    Ok(())
}

pub fn data_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("ALT_DATA_DIR") {
        return path.into();
    }
    if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(path).join("alt-cli");
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local/share/alt-cli")
}

pub fn private_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
