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
        self.validate_api_key_env()?;
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
        self.validate_api_key_env()?;
        self.api_key_env
            .as_ref()
            .map(|name| {
                // Do not attach VarError: its non-Unicode variant contains the
                // environment value, and the configured name may be a pasted key.
                std::env::var(name).map_err(|_| {
                    anyhow::anyhow!(
                        "API key environment variable is missing or is not valid Unicode. Set it in your terminal before starting Alt."
                    )
                })
            })
            .transpose()
    }

    fn validate_api_key_env(&self) -> Result<()> {
        if let Some(name) = &self.api_key_env {
            let mut bytes = name.bytes();
            let valid_start = bytes
                .next()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_');
            if !valid_start || !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_') {
                bail!(
                    "Use an environment variable name such as ALT_API_KEY: start with a letter or underscore, then use letters, digits or underscores. Do not paste the key itself."
                );
            }
        }
        Ok(())
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
        let _generation = crate::storage::StateWriteGuard::acquire(root)?;
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemePreset {
    Graphite,
    Aurora,
    Ember,
    Daylight,
    HighContrast,
    #[default]
    #[serde(other)]
    Lagoon,
}

impl ThemePreset {
    pub const ALL: [Self; 6] = [
        Self::Lagoon,
        Self::Graphite,
        Self::Aurora,
        Self::Ember,
        Self::Daylight,
        Self::HighContrast,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Lagoon => "Lagoon",
            Self::Graphite => "Graphite",
            Self::Aurora => "Aurora",
            Self::Ember => "Ember",
            Self::Daylight => "Daylight",
            Self::HighContrast => "High contrast",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Lagoon => "Deep blue surfaces with a clear teal accent",
            Self::Graphite => "Neutral charcoal with a cool silver accent",
            Self::Aurora => "Midnight violet with soft purple highlights",
            Self::Ember => "Warm dark surfaces with an amber accent",
            Self::Daylight => "Light surfaces with dark text and teal highlights",
            Self::HighContrast => "Black surfaces, bright text and strong outlines",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayoutPreset {
    Sidebar,
    Tabs,
    Focus,
    #[default]
    #[serde(other)]
    Auto,
}

impl LayoutPreset {
    pub const ALL: [Self; 4] = [Self::Auto, Self::Sidebar, Self::Tabs, Self::Focus];

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Automatic",
            Self::Sidebar => "Sidebar",
            Self::Tabs => "Top tabs",
            Self::Focus => "Focus",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Auto => "Sidebar on wide screens; tabs on smaller screens",
            Self::Sidebar => "Keep navigation at the left; narrow screens use tabs",
            Self::Tabs => "Put navigation above your workspace",
            Self::Focus => "More room for your work; Tab reveals navigation, Ctrl+P finds pages",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub theme: ThemePreset,
    pub layout: LayoutPreset,
    pub decorations: bool,
    pub accent: Option<String>,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: ThemePreset::default(),
            layout: LayoutPreset::default(),
            decorations: true,
            accent: None,
        }
    }
}

impl Appearance {
    /// Startup only needs presentation. Keep this read bounded and leave any
    /// recovery, validation and writes to the normal application loader.
    pub fn load_for_startup(root: &Path) -> Self {
        use std::io::Read;
        const LIMIT: u64 = 256 * 1024;
        let loaded = (|| -> Option<Self> {
            let file = std::fs::File::open(root.join("preferences.toml")).ok()?;
            let mut text = String::new();
            file.take(LIMIT + 1).read_to_string(&mut text).ok()?;
            if text.len() > LIMIT as usize {
                return None;
            }
            let value: toml::Value = toml::from_str(&text).ok()?;
            Some(Self::from_value(value.get("appearance")?.clone()))
        })();
        loaded.unwrap_or_default()
    }

    fn from_value(value: toml::Value) -> Self {
        let mut appearance: Self = value.try_into().unwrap_or_default();
        if let Some(accent) = appearance.accent.take() {
            // An invalid cosmetic value must not discard connection, runtime,
            // project or access preferences from the same file.
            let _ = appearance.set_accent(&accent);
        }
        appearance
    }

    pub fn accent_rgb(&self) -> Option<(u8, u8, u8)> {
        let text = self.accent.as_ref()?.strip_prefix('#')?;
        if text.len() != 6 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        Some((
            u8::from_str_radix(&text[0..2], 16).ok()?,
            u8::from_str_radix(&text[2..4], 16).ok()?,
            u8::from_str_radix(&text[4..6], 16).ok()?,
        ))
    }

    pub fn set_accent(&mut self, text: &str) -> Result<()> {
        let text = text.trim();
        if text.is_empty() || text.eq_ignore_ascii_case("default") {
            self.accent = None;
            return Ok(());
        }
        let hex = text.strip_prefix('#').unwrap_or(text);
        if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            bail!(
                "Enter six hexadecimal digits, such as #51D3CA, or leave blank for the theme default"
            );
        }
        self.accent = Some(format!("#{}", hex.to_ascii_uppercase()));
        Ok(())
    }
}

fn deserialize_appearance<'de, D>(deserializer: D) -> std::result::Result<Appearance, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Appearance::from_value(toml::Value::deserialize(
        deserializer,
    )?))
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
    #[serde(deserialize_with = "deserialize_appearance")]
    pub appearance: Appearance,
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
            appearance: Appearance::default(),
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
        let _generation = crate::storage::StateWriteGuard::acquire(root)?;
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
    let _generation = crate::storage::write_guard_for_path(path)?;
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

#[cfg(test)]
mod tests {
    use super::{Appearance, Config, LayoutPreset, Preferences, Profile, Provider, ThemePreset};
    use std::collections::BTreeMap;

    fn profile(name: Option<&str>) -> Profile {
        Profile {
            provider: Provider::Openai,
            endpoint: "http://127.0.0.1:1234/v1".into(),
            model: "test-model".into(),
            context_tokens: 8192,
            max_turns: 12,
            uncensored: false,
            api_key_env: name.map(str::to_owned),
            local_model: None,
            inference: None,
        }
    }

    #[test]
    fn appearance_migration_preserves_existing_preferences_and_recovers_cosmetic_errors() {
        let root = tempfile::tempdir().unwrap();
        for appearance in [
            "",
            "\n[appearance]\ntheme = 'future-theme'\nlayout = 'future-layout'",
            "\n[appearance]\ntheme = 12\nlayout = false",
            "\nappearance = 'invalid'",
        ] {
            std::fs::write(
                root.path().join("preferences.toml"),
                format!("context_tokens = 16384\nmax_turns = 24\nmouse = false\nproject = '/retained/project'\n{appearance}"),
            ).unwrap();
            let loaded = Preferences::load(root.path()).unwrap();
            assert_eq!(loaded.context_tokens, 16384);
            assert_eq!(loaded.max_turns, 24);
            assert!(!loaded.mouse);
            assert_eq!(loaded.project, std::path::Path::new("/retained/project"));
            assert_eq!(loaded.appearance, Appearance::default());
        }

        std::fs::write(
            root.path().join("preferences.toml"),
            "context_tokens = 32768\n[appearance]\ntheme = 'aurora'\naccent = '#🦀0000'\n",
        )
        .unwrap();
        let loaded = Preferences::load(root.path()).unwrap();
        assert_eq!(loaded.context_tokens, 32768);
        assert_eq!(loaded.appearance.theme, ThemePreset::Aurora);
        assert_eq!(loaded.appearance.accent, None);
    }

    #[test]
    fn appearance_presets_layouts_and_accent_persist_without_changing_runtime_preferences() {
        let root = tempfile::tempdir().unwrap();
        let mut preferences = Preferences {
            mouse: false,
            context_tokens: 16384,
            max_turns: 24,
            engine_path: Some("/custom/engine".into()),
            ..Preferences::default()
        };
        preferences.appearance.set_accent("7aa2f7").unwrap();
        for theme in ThemePreset::ALL {
            for layout in LayoutPreset::ALL {
                for decorations in [false, true] {
                    preferences.appearance.theme = theme;
                    preferences.appearance.layout = layout;
                    preferences.appearance.decorations = decorations;
                    preferences.save(root.path()).unwrap();
                    let restored = Preferences::load(root.path()).unwrap();
                    assert_eq!(restored.appearance, preferences.appearance);
                    assert_eq!(restored.appearance.accent_rgb(), Some((122, 162, 247)));
                    assert_eq!(restored.context_tokens, 16384);
                    assert_eq!(restored.max_turns, 24);
                    assert!(!restored.mouse);
                    assert_eq!(restored.engine_path, preferences.engine_path);
                    assert_eq!(restored.project, preferences.project);
                    assert_eq!(
                        Appearance::load_for_startup(root.path()),
                        restored.appearance
                    );
                }
            }
        }
    }

    #[test]
    fn startup_appearance_read_is_bounded_and_does_not_repair_files() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("preferences.toml");
        for text in [
            "[appearance]\ntheme = 'daylight'\n#".to_owned() + &"x".repeat(256 * 1024),
            "invalid toml = [".into(),
        ] {
            std::fs::write(&path, &text).unwrap();
            assert_eq!(
                Appearance::load_for_startup(root.path()),
                Appearance::default()
            );
            assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        }
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn portable_api_key_variable_names_round_trip_unchanged() {
        let root = tempfile::tempdir().unwrap();
        let names = ["ALT_API_KEY", "api_key", "_KEY", "_", "KEY2"];
        let mut config = Config::default();
        for name in names {
            config.upsert(name.into(), profile(Some(name))).unwrap();
        }
        config.save(root.path()).unwrap();
        let restored = Config::read(root.path()).unwrap();
        for name in names {
            assert_eq!(restored.profiles[name].api_key_env.as_deref(), Some(name));
        }
        assert_eq!(profile(None).key().unwrap(), None);
        assert_eq!(
            profile(Some("PATH")).key().unwrap(),
            std::env::var("PATH").ok()
        );
    }

    #[test]
    fn pasted_key_syntax_is_rejected_before_persistence_without_echo() {
        let root = tempfile::tempdir().unwrap();
        Config::create(root.path(), profile(None)).unwrap();
        let original = std::fs::read(root.path().join("config.toml")).unwrap();
        for name in [
            "sk-test-private-token",
            "Bearer private-token",
            "NAME=private-token",
            " leading",
            "9NAME",
            "KEY.WITH.DOT",
            "",
            "密钥",
            "KEY\0private-token",
        ] {
            let candidate = profile(Some(name));
            let error = candidate.validate().unwrap_err().to_string();
            assert!(error.starts_with("Use an environment variable name"));
            assert!(!error.contains("private-token"));
            assert_eq!(candidate.key().unwrap_err().to_string(), error);
            let mut config = Config::read(root.path()).unwrap();
            assert!(
                config
                    .upsert("candidate".into(), candidate.clone())
                    .is_err()
            );
            config.profiles.insert("candidate".into(), candidate);
            assert!(config.save(root.path()).is_err());
            assert_eq!(
                std::fs::read(root.path().join("config.toml")).unwrap(),
                original
            );
        }
    }

    #[test]
    fn manually_saved_invalid_variable_and_missing_variable_errors_are_redacted() {
        let root = tempfile::tempdir().unwrap();
        let config = Config {
            default_profile: "local".into(),
            profiles: BTreeMap::from([("local".into(), profile(Some("sk-private-token")))]),
        };
        std::fs::write(
            root.path().join("config.toml"),
            toml::to_string(&config).unwrap(),
        )
        .unwrap();
        let error = format!("{:#}", Config::read(root.path()).unwrap_err());
        assert!(!error.contains("sk-private-token"));
        let missing = format!("ALT_CONFIG_MISSING_KEY_{}", std::process::id());
        let error = format!("{:#}", profile(Some(&missing)).key().unwrap_err());
        assert!(error.contains("environment variable is missing"));
        assert!(!error.contains(&missing));
    }

    #[cfg(unix)]
    #[test]
    fn non_unicode_api_key_value_is_not_exposed_by_error_chain() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt, process::Command};
        const NAME: &str = "ALT_CONFIG_NON_UNICODE_KEY_TEST";
        if std::env::var_os(NAME).is_some() {
            let error = format!("{:#?}", profile(Some(NAME)).key().unwrap_err());
            assert!(error.contains("environment variable is missing or is not valid Unicode"));
            assert!(!error.contains("private-token"));
            assert!(!error.contains(NAME));
        } else {
            // Child-only environment avoids mutating process globals while Rust
            // runs unrelated tests on other threads.
            let output = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "config::tests::non_unicode_api_key_value_is_not_exposed_by_error_chain",
                    "--nocapture",
                ])
                .env(NAME, OsString::from_vec(b"private-token\xff".to_vec()))
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
