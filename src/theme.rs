use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const ENV_THEME: &str = "JETCTX_THEME";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Theme {
    pub name: String,
    #[serde(default)]
    pub base: BasePalette,
    #[serde(default)]
    pub semantic: SemanticPalette,
    #[serde(default)]
    pub prompt: PromptPalette,
    #[serde(default)]
    pub tmux: TmuxPalette,
}

impl Theme {
    pub fn from_path(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read theme file: {}", path.display()))?;

        let theme: Theme = toml::from_str(&raw)
            .with_context(|| format!("failed to parse theme TOML: {}", path.display()))?;

        Ok(theme)
    }

    pub fn resolve(
        explicit_theme: Option<&str>,
        config_theme: Option<&str>,
        search_dirs: &[PathBuf],
    ) -> Result<Self> {
        let env_theme = env::var(ENV_THEME).ok();

        let theme_name = explicit_theme
            .and_then(non_empty)
            .or_else(|| env_theme.as_deref().and_then(non_empty))
            .or_else(|| config_theme.and_then(non_empty))
            .unwrap_or("nightowl");

        Self::load_named(theme_name, search_dirs)
    }

    pub fn load_named(name: &str, search_dirs: &[PathBuf]) -> Result<Self> {
        let path = find_theme_path(name, search_dirs)
            .with_context(|| format!("failed to locate theme '{name}'"))?;

        Self::from_path(&path)
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BasePalette {
    #[serde(default)]
    pub bg: Option<String>,
    #[serde(default)]
    pub fg: Option<String>,
    #[serde(default)]
    pub muted: Option<String>,
    #[serde(default)]
    pub subtle: Option<String>,
    #[serde(default)]
    pub surface: Option<String>,
    #[serde(default)]
    pub surface_alt: Option<String>,
    #[serde(default)]
    pub border: Option<String>,
    #[serde(default)]
    pub border_active: Option<String>,
    #[serde(default)]
    pub selection: Option<String>,
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SemanticPalette {
    #[serde(default)]
    pub ok: Option<String>,
    #[serde(default)]
    pub warn: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PromptPalette {
    #[serde(default)]
    pub directory: Option<String>,
    #[serde(default)]
    pub status_ok: Option<String>,
    #[serde(default)]
    pub status_error: Option<String>,
    #[serde(default)]
    pub duration: Option<String>,
    #[serde(default)]
    pub success_symbol: Option<String>,
    #[serde(default)]
    pub error_symbol: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TmuxPalette {
    #[serde(default)]
    pub status_bg: Option<String>,
    #[serde(default)]
    pub status_fg: Option<String>,
    #[serde(default)]
    pub status_muted: Option<String>,
    #[serde(default)]
    pub session_bg: Option<String>,
    #[serde(default)]
    pub session_fg: Option<String>,
    #[serde(default)]
    pub window_active_bg: Option<String>,
    #[serde(default)]
    pub window_active_fg: Option<String>,
    #[serde(default)]
    pub window_inactive_bg: Option<String>,
    #[serde(default)]
    pub window_inactive_fg: Option<String>,
    #[serde(default)]
    pub segment_info_bg: Option<String>,
    #[serde(default)]
    pub segment_info_fg: Option<String>,
    #[serde(default)]
    pub segment_warn_bg: Option<String>,
    #[serde(default)]
    pub segment_warn_fg: Option<String>,
    #[serde(default)]
    pub segment_error_bg: Option<String>,
    #[serde(default)]
    pub segment_error_fg: Option<String>,
    #[serde(default)]
    pub segment_time_bg: Option<String>,
    #[serde(default)]
    pub segment_time_fg: Option<String>,
}

pub fn default_theme_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(xdg_home) = env::var("XDG_CONFIG_HOME") {
        dirs.push(PathBuf::from(xdg_home).join("jetctx").join("themes"));
    } else if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".config").join("jetctx").join("themes"));
    }

    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        dirs.push(PathBuf::from(manifest_dir).join("themes"));
    }

    dirs
}

pub fn find_theme_path(name: &str, search_dirs: &[PathBuf]) -> Result<PathBuf> {
    let filename = format!("{name}.toml");

    for dir in search_dirs {
        let candidate = dir.join(&filename);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    anyhow::bail!(
        "theme '{}' not found in any search directory: {}",
        name,
        search_dirs
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}
