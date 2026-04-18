use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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
        let path = Self::resolve_path(explicit_theme, config_theme, search_dirs)?;
        Self::from_path(&path)
    }

    pub fn resolve_path(
        explicit_theme: Option<&str>,
        config_theme: Option<&str>,
        search_dirs: &[PathBuf],
    ) -> Result<PathBuf> {
        let env_theme = env::var(ENV_THEME).ok();

        let theme_name = explicit_theme
            .and_then(non_empty)
            .or_else(|| env_theme.as_deref().and_then(non_empty))
            .or_else(|| config_theme.and_then(non_empty))
            .unwrap_or("nightowl");

        find_theme_path(theme_name, search_dirs)
            .with_context(|| format!("failed to locate theme '{theme_name}'"))
    }

    pub fn load_named(name: &str, search_dirs: &[PathBuf]) -> Result<Self> {
        let path = find_theme_path(name, search_dirs)
            .with_context(|| format!("failed to locate theme '{name}'"))?;

        Self::from_path(&path)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ThemeSupportReport {
    pub supported_fields_present: Vec<String>,
    pub ignored_fields_present: Vec<String>,
}

pub fn analyze_theme_file(path: &Path) -> Result<ThemeSupportReport> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read theme file: {}", path.display()))?;
    let value: toml::Value = toml::from_str(&raw)
        .with_context(|| format!("failed to parse theme TOML: {}", path.display()))?;

    let mut present_fields = BTreeSet::new();
    collect_leaf_keys(None, &value, &mut present_fields);

    let supported_fields = supported_theme_fields();
    let supported_fields_present = present_fields
        .iter()
        .filter(|field| supported_fields.contains(field.as_str()))
        .cloned()
        .collect();
    let ignored_fields_present = present_fields
        .iter()
        .filter(|field| !supported_fields.contains(field.as_str()))
        .cloned()
        .collect();

    Ok(ThemeSupportReport {
        supported_fields_present,
        ignored_fields_present,
    })
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BasePalette {
    #[serde(default)]
    pub bg: Option<String>,
    #[serde(default)]
    pub fg: Option<String>,
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
    pub repo: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub dirty: Option<String>,
    #[serde(default)]
    pub status_ok: Option<String>,
    #[serde(default)]
    pub status_error: Option<String>,
    #[serde(default)]
    pub duration: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub success_symbol: Option<String>,
    #[serde(default)]
    pub error_symbol: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TmuxPalette {
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

fn collect_leaf_keys(prefix: Option<&str>, value: &toml::Value, out: &mut BTreeSet<String>) {
    match value {
        toml::Value::Table(table) => {
            for (key, value) in table {
                let next = match prefix {
                    Some(prefix) => format!("{prefix}.{key}"),
                    None => key.clone(),
                };
                collect_leaf_keys(Some(&next), value, out);
            }
        }
        _ => {
            if let Some(prefix) = prefix {
                out.insert(prefix.to_string());
            }
        }
    }
}

fn supported_theme_fields() -> BTreeSet<&'static str> {
    BTreeSet::from([
        "name",
        "base.bg",
        "base.fg",
        "semantic.ok",
        "semantic.warn",
        "semantic.error",
        "prompt.directory",
        "prompt.repo",
        "prompt.branch",
        "prompt.dirty",
        "prompt.status_ok",
        "prompt.status_error",
        "prompt.duration",
        "prompt.symbol",
        "prompt.success_symbol",
        "prompt.error_symbol",
        "tmux.segment_info_bg",
        "tmux.segment_info_fg",
        "tmux.segment_warn_bg",
        "tmux.segment_warn_fg",
        "tmux.segment_error_bg",
        "tmux.segment_error_fg",
        "tmux.segment_time_bg",
        "tmux.segment_time_fg",
    ])
}
