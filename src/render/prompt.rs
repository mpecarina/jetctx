use std::path::Path;

use serde::Serialize;

use crate::cache::{self, GitCacheEntry};
use crate::cli::{OutputFormat, PromptArgs};
use crate::config::Config;
use crate::detect::{git, project};
use crate::theme::Theme;

/// First-pass prompt renderer.
///
/// Current scope:
/// - schema-aligned with `config.toml` prompt fields where practical
/// - fast and side-effect free
/// - project and git detection are centralized through `crate::detect::*`
/// - supports both text and JSON output
///
/// Render model for v0.1:
/// - line 1: optional status + cwd + optional git + optional duration
/// - line 2: prompt symbol
///
/// Example text output:
///   ~/repo △ feature * ◄ 842ms
///   ◎
///
/// Example error output:
///   status=1 ~/repo △ feature * ◄ 842ms
///   ○
#[derive(Debug, Clone, Serialize)]
pub struct PromptRenderModel {
    pub exit_code: i32,
    pub cwd: String,
    pub git_branch: Option<String>,
    pub git_dirty: bool,
    pub duration_ms: Option<u64>,
    pub show_duration: bool,
    pub duration_min_ms: u64,
    pub symbol: String,
    pub segments: Vec<String>,
    pub rendered: String,
}

pub fn render(args: &PromptArgs, config: &Config, theme: &Theme) -> String {
    match args.format {
        OutputFormat::Text => render_text(args, config, theme),
        OutputFormat::Json => render_json(args, config, theme),
    }
}

pub fn render_model(args: &PromptArgs, config: &Config, theme: &Theme) -> PromptRenderModel {
    let cwd_path = args.cwd.as_deref();
    let cwd = render_cwd(cwd_path, config);
    let project_context = detect_project_context(cwd_path, config);
    let duration_visible = should_show_duration(args, config);

    let symbol = if args.exit_code == 0 {
        success_symbol(theme)
    } else {
        error_symbol(theme)
    };

    let mut segments = Vec::new();

    segments.push(cwd.clone());

    if config.prompt.show_git {
        if let Some(git_segment) = render_git_segment(&project_context, config) {
            segments.push(git_segment);
        }
    }

    if duration_visible {
        if let Some(duration_ms) = args.duration_ms {
            segments.push(format!("◄ {}ms", duration_ms));
        }
    }

    let mut rendered = String::new();
    if !segments.is_empty() {
        rendered.push_str(&segments.join(" "));
        rendered.push('\n');
    }
    rendered.push_str(&symbol);

    PromptRenderModel {
        exit_code: args.exit_code,
        cwd,
        git_branch: project_context
            .git
            .as_ref()
            .and_then(|git| git.branch.clone()),
        git_dirty: project_context
            .git
            .as_ref()
            .map(|git| git.dirty)
            .unwrap_or(false),
        duration_ms: args.duration_ms,
        show_duration: duration_visible,
        duration_min_ms: config.prompt.duration_min_ms,
        symbol,
        segments,
        rendered,
    }
}

fn render_text(args: &PromptArgs, config: &Config, theme: &Theme) -> String {
    render_model(args, config, theme).rendered
}

fn render_json(args: &PromptArgs, config: &Config, theme: &Theme) -> String {
    let model = render_model(args, config, theme);
    serde_json::to_string_pretty(&model).unwrap_or_else(|_| {
        format!(
            "{{\"error\":\"failed to serialize prompt model\",\"cwd\":{:?}}}",
            model.cwd
        )
    })
}

fn should_show_duration(args: &PromptArgs, config: &Config) -> bool {
    if !config.prompt.show_duration {
        return false;
    }

    match args.duration_ms {
        Some(ms) => ms >= config.prompt.duration_min_ms,
        None => false,
    }
}

#[derive(Debug, Clone, Default)]
struct ProjectContext {
    git: Option<git::GitContext>,
}

fn detect_project_context(cwd: Option<&Path>, config: &Config) -> ProjectContext {
    let Some(cwd) = cwd else {
        return ProjectContext::default();
    };

    if let Some(cached) = load_cached_project_context(cwd, config) {
        return cached;
    }

    detect_live_project_context(cwd)
}

fn detect_live_project_context(cwd: &Path) -> ProjectContext {
    let git = project::detect(cwd).as_ref().and_then(|info| {
        if info.has_git_marker() || matches!(info.kind, project::ProjectKind::Git) {
            git::detect(&info.root).ok().flatten()
        } else {
            None
        }
    });

    ProjectContext { git }
}

fn load_cached_project_context(cwd: &Path, config: &Config) -> Option<ProjectContext> {
    let cached = cache::load_or_refresh_project_entry(cwd, config)
        .ok()
        .flatten()?;
    let git_info = cached
        .git
        .as_ref()
        .map(|git_cache| git_from_cache(cwd, git_cache));

    Some(ProjectContext { git: git_info })
}

fn git_from_cache(cwd: &Path, cached: &GitCacheEntry) -> git::GitContext {
    let repo_root = project::detect(cwd)
        .map(|info| info.root)
        .unwrap_or_else(|| cwd.to_path_buf());

    git::GitContext {
        repo_root: repo_root.clone(),
        git_dir: repo_root.join(".git"),
        branch: cached.branch.clone(),
        dirty: cached.dirty,
        head_oid_short: None,
        staged: 0,
        modified: 0,
        untracked: 0,
        conflicted: 0,
        ahead: 0,
        behind: 0,
        detached: false,
        head_mtime: None,
        index_mtime: None,
    }
}

fn render_git_segment(project: &ProjectContext, _config: &Config) -> Option<String> {
    let git = project.git.as_ref()?;

    if let Some(branch) = git.branch.as_deref() {
        let mut branch_segment = String::from("△ ");
        branch_segment.push_str(branch);

        if git.dirty {
            branch_segment.push_str(" *");
        }

        Some(branch_segment)
    } else {
        None
    }
}

fn success_symbol(theme: &Theme) -> String {
    theme
        .prompt
        .success_symbol
        .clone()
        .unwrap_or_else(|| "◎".to_string())
}

fn error_symbol(theme: &Theme) -> String {
    theme
        .prompt
        .error_symbol
        .clone()
        .unwrap_or_else(|| "○".to_string())
}

fn render_cwd(cwd: Option<&Path>, config: &Config) -> String {
    let Some(cwd) = cwd else {
        return ".".to_string();
    };
    let _ = config;
    cwd.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::PromptArgs;
    use crate::config::Config;
    use crate::theme::Theme;

    fn test_theme() -> Theme {
        Theme {
            name: "test".to_string(),
            kind: Some("dark".to_string()),
            base: Default::default(),
            accent: Default::default(),
            semantic: Default::default(),
            prompt: crate::theme::PromptPalette {
                success_symbol: Some("◎".to_string()),
                error_symbol: Some("○".to_string()),
                ..Default::default()
            },
            tmux: Default::default(),
        }
    }

    fn test_config() -> Config {
        let mut config = Config::default();
        config.prompt.duration_min_ms = 400;
        config.prompt.cwd_components = 2;
        config
    }

    #[test]
    fn renders_success_prompt_without_status_or_duration() {
        let args = PromptArgs {
            cwd: Some(PathBuf::from("/tmp/example")),
            exit_code: 0,
            duration_ms: Some(10),
            format: OutputFormat::Text,
        };

        let rendered = render(&args, &test_config(), &test_theme());
        assert_eq!(rendered, "/tmp/example\n◎");
    }

    #[test]
    fn renders_error_and_duration() {
        let args = PromptArgs {
            cwd: Some(PathBuf::from("/tmp/example")),
            exit_code: 1,
            duration_ms: Some(842),
            format: OutputFormat::Text,
        };

        let rendered = render(&args, &test_config(), &test_theme());
        assert_eq!(rendered, "status=1 /tmp/example ◄ 842ms\n○");
    }

    #[test]
    fn truncates_long_paths() {
        let rendered = truncate_path(Path::new("/a/b/c/d"), 2);
        assert_eq!(rendered, "/c/d");
    }

    #[test]
    fn renders_json_output() {
        let args = PromptArgs {
            cwd: Some(PathBuf::from("/tmp/example")),
            exit_code: 0,
            duration_ms: Some(500),
            format: OutputFormat::Json,
        };

        let rendered = render(&args, &test_config(), &test_theme());
        assert!(rendered.contains("\"cwd\": \"/tmp/example\""));
        assert!(rendered.contains("\"show_duration\": true"));
    }
}
