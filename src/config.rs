use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default)]
    pub prompt: PromptConfig,

    #[serde(default)]
    pub tmux: TmuxConfig,

    #[serde(default)]
    pub update: UpdateConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            prompt: PromptConfig::default(),
            tmux: TmuxConfig::default(),
            update: UpdateConfig::default(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        Self::load_with_env(env::var_os("HOME"), env::var("JETCTX_CONFIG").ok())
    }

    pub fn load_with_env(
        home: Option<std::ffi::OsString>,
        override_path: Option<String>,
    ) -> Result<Self> {
        let mut config = if let Some(path) = override_path {
            Self::from_path(path)?
        } else if let Some(path) = default_config_path(home.as_deref()) {
            if path.exists() {
                Self::from_path(path)?
            } else {
                Self::default()
            }
        } else {
            Self::default()
        };

        config.apply_env_overrides();

        Ok(config)
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;

        let config: Self = toml::from_str(&contents)
            .with_context(|| format!("failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(theme) = env::var("JETCTX_THEME") {
            let trimmed = theme.trim();
            if !trimmed.is_empty() {
                self.theme = trimmed.to_string();
            }
        }
    }

    pub fn default_config_path() -> Option<PathBuf> {
        default_config_path(env::var_os("HOME").as_deref())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PromptConfig {
    #[serde(default = "default_true")]
    pub show_git: bool,

    #[serde(default = "default_true")]
    pub show_duration: bool,

    #[serde(default = "default_duration_min_ms")]
    pub duration_min_ms: u64,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            show_git: true,
            show_duration: true,
            duration_min_ms: default_duration_min_ms(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TmuxConfig {
    #[serde(default = "default_true")]
    pub show_memory: bool,

    #[serde(default = "default_battery_symbol")]
    pub battery_symbol: String,

    #[serde(default = "default_memory_symbol")]
    pub memory_symbol: String,

    #[serde(default = "default_time_symbol")]
    pub time_symbol: String,
}

impl Default for TmuxConfig {
    fn default() -> Self {
        Self {
            show_memory: true,
            battery_symbol: default_battery_symbol(),
            memory_symbol: default_memory_symbol(),
            time_symbol: default_time_symbol(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateConfig {
    #[serde(default = "default_host_ttl_seconds")]
    pub host_ttl_seconds: u64,

    #[serde(default = "default_project_ttl_seconds")]
    pub project_ttl_seconds: u64,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            host_ttl_seconds: default_host_ttl_seconds(),
            project_ttl_seconds: default_project_ttl_seconds(),
        }
    }
}

fn default_config_path(home: Option<&std::ffi::OsStr>) -> Option<PathBuf> {
    let home = home?;
    let mut path = PathBuf::from(home);
    path.push(".config");
    path.push("jetctx");
    path.push("config.toml");
    Some(path)
}

fn default_theme() -> String {
    "nightowl".to_string()
}

fn default_true() -> bool {
    true
}

fn default_duration_min_ms() -> u64 {
    400
}

fn default_host_ttl_seconds() -> u64 {
    15
}

fn default_project_ttl_seconds() -> u64 {
    3
}

fn default_battery_symbol() -> String {
    "BAT".to_string()
}

fn default_memory_symbol() -> String {
    "MEM".to_string()
}

fn default_time_symbol() -> String {
    "◷".to_string()
}
