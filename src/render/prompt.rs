use std::path::Path;

use serde::Serialize;

use crate::cache::{self, GitCacheEntry};
use crate::cli::PromptArgs;
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
    render_text(args, config, theme)
}

pub fn render_model(args: &PromptArgs, config: &Config, theme: &Theme) -> PromptRenderModel {
    let resolved_cwd = args.cwd.clone().or_else(|| std::env::current_dir().ok());
    let cwd_path = resolved_cwd.as_deref();
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
    let model = render_model(args, config, theme);
    let mut segments = Vec::new();

    segments.push(paint(&model.cwd, prompt_directory_color(theme)));

    if let Some(branch) = model.git_branch.as_deref() {
        let mut git_segment = vec![
            paint("△", prompt_git_marker_color(theme)),
            paint(branch, prompt_branch_color(theme)),
        ];
        if model.git_dirty {
            git_segment.push(paint("*", prompt_dirty_color(theme)));
        }
        segments.push(git_segment.join(" "));
    }

    if model.show_duration {
        if let Some(duration_ms) = model.duration_ms {
            segments.push(paint(
                &format!("◄ {}ms", duration_ms),
                prompt_duration_color(theme),
            ));
        }
    }

    let mut rendered = String::new();
    if !segments.is_empty() {
        rendered.push_str(&segments.join(" "));
        rendered.push('\n');
    }
    rendered.push_str(&paint(
        &model.symbol,
        prompt_symbol_color(theme, model.exit_code),
    ));
    rendered
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
    // Git and language/package roots are independent. Detecting git directly
    // from cwd preserves repository context inside nested monorepo packages.
    let git = git::detect(cwd).ok().flatten();

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

fn paint(text: &str, color: Option<&str>) -> String {
    let escaped = escape_prompt_text(text);
    match color.and_then(non_empty) {
        Some(color) => format!("%F{{{color}}}{escaped}%f"),
        None => escaped,
    }
}

fn escape_prompt_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());

    for ch in text.chars() {
        match ch {
            // zsh prompt escapes are introduced with %. For characters that
            // prompt_subst would execute, emit a non-recursive parameter
            // expansion that evaluates to the literal character.
            '%' => escaped.push_str("%%"),
            '\\' => escaped.push_str("${:-\\\\}"),
            '$' => escaped.push_str("${:-\\$}"),
            '`' => escaped.push_str("${:-\\`}"),
            _ => escaped.push(ch),
        }
    }

    escaped
}

fn prompt_directory_color(theme: &Theme) -> Option<&str> {
    theme
        .prompt
        .directory
        .as_deref()
        .or(theme.tmux.segment_info_bg.as_deref())
        .or(theme.base.fg.as_deref())
}

fn prompt_branch_color(theme: &Theme) -> Option<&str> {
    theme
        .prompt
        .branch
        .as_deref()
        .or(theme.tmux.segment_info_fg.as_deref())
        .or(theme.prompt.repo.as_deref())
        .or(theme.tmux.segment_info_bg.as_deref())
        .or(theme.base.fg.as_deref())
}

fn prompt_git_marker_color(theme: &Theme) -> Option<&str> {
    theme
        .prompt
        .repo
        .as_deref()
        .or(theme.tmux.segment_info_bg.as_deref())
        .or(theme.base.fg.as_deref())
}

fn prompt_dirty_color(theme: &Theme) -> Option<&str> {
    theme
        .prompt
        .dirty
        .as_deref()
        .or(theme.semantic.warn.as_deref())
        .or(theme.tmux.segment_warn_bg.as_deref())
        .or(theme.base.fg.as_deref())
}

fn prompt_duration_color(theme: &Theme) -> Option<&str> {
    theme
        .prompt
        .duration
        .as_deref()
        .or(theme.tmux.segment_time_bg.as_deref())
        .or(theme.semantic.warn.as_deref())
}

fn prompt_symbol_color(theme: &Theme, exit_code: i32) -> Option<&str> {
    theme.prompt.symbol.as_deref().or_else(|| {
        if exit_code == 0 {
            theme
                .prompt
                .status_ok
                .as_deref()
                .or(theme.tmux.segment_info_bg.as_deref())
                .or(theme.semantic.ok.as_deref())
        } else {
            theme
                .prompt
                .status_error
                .as_deref()
                .or(theme.tmux.segment_error_bg.as_deref())
                .or(theme.semantic.error.as_deref())
        }
    })
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
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
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::cli::{OutputFormat, PromptArgs};
    use crate::config::Config;
    use crate::theme::Theme;

    fn test_theme() -> Theme {
        Theme {
            name: "test".to_string(),
            base: Default::default(),
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
        assert_eq!(rendered, "/tmp/example ◄ 842ms\n○");
    }

    #[test]
    fn escapes_zsh_prompt_substitutions_without_changing_plain_text() {
        assert_eq!(escape_prompt_text("plain/path"), "plain/path");
        assert_eq!(
            escape_prompt_text("$(>PWNED) `cmd` \\ 100%"),
            "${:-\\$}(>PWNED) ${:-\\`}cmd${:-\\`} ${:-\\\\} 100%%"
        );
    }

    #[test]
    fn detects_git_above_a_nested_project_marker() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "jetctx-prompt-test-{}-{unique}",
            std::process::id()
        ));
        let package = root.join("packages/app");
        fs::create_dir_all(root.join(".git")).expect("git dir should be created");
        fs::create_dir_all(&package).expect("package dir should be created");
        fs::write(root.join(".git/HEAD"), "ref: refs/heads/feature/deep\n")
            .expect("HEAD should be written");
        fs::write(package.join("package.json"), "{}\n").expect("package marker should be written");

        let context = detect_live_project_context(&package);
        assert_eq!(
            context.git.and_then(|git| git.branch),
            Some("feature/deep".to_string())
        );

        fs::remove_dir_all(root).expect("temporary project should be removed");
    }
}
