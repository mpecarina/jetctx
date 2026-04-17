mod cache;
mod cli;
mod config;
mod detect;
mod render;
mod theme;

use std::path::PathBuf;
use std::process;

use anyhow::{Context, Result};
use cache::{
    load_host_entry, load_or_refresh_host_entry, load_or_refresh_project_entry, refresh_host_cache,
    refresh_project_cache, ProjectCacheEntry,
};
use cli::{
    Cli, Command, InspectArgs, InspectFormatArgs, InspectProjectArgs, InspectTarget,
    InspectThemeArgs, OutputFormat, PromptArgs, TmuxArgs, UpdateArgs, UpdateScope,
};
use config::Config;
use render::prompt;
use render::tmux::{self, TmuxContext, TmuxTarget};
use theme::{default_theme_search_dirs, Theme};

fn main() {
    if let Err(err) = run() {
        eprintln!("jetctx: {err:#}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse_from_env();
    let config = Config::load().context("failed to load jetctx config")?;

    match cli.command {
        Command::Prompt(args) => handle_prompt(&config, &args)?,
        Command::Tmux(args) => handle_tmux(&config, &args)?,
        Command::Update(args) => handle_update(&config, &args)?,
        Command::Inspect(args) => handle_inspect(&config, &args)?,
        Command::Doctor => handle_doctor(&config)?,
        Command::Version => println!("{}", env!("CARGO_PKG_VERSION")),
    }

    Ok(())
}

fn handle_prompt(config: &Config, args: &PromptArgs) -> Result<()> {
    let theme = load_theme(config, None)?;
    let rendered = prompt::render(args, config, &theme);
    print_output(&rendered, None, args.format)
}

fn handle_tmux(config: &Config, args: &TmuxArgs) -> Result<()> {
    let theme = load_theme(config, None)?;
    let host_cache = load_or_refresh_host_entry(config)?;

    let context = tmux_context(args.cwd.as_ref(), &host_cache);
    let rendered = tmux::render(TmuxTarget::Right, &context, config, &theme);
    print_output(&rendered, Some(&context), args.format)
}

fn handle_update(config: &Config, args: &UpdateArgs) -> Result<()> {
    match &args.scope {
        UpdateScope::Host(update_args) => {
            let entry = if update_args.force {
                refresh_host_cache(config)?
            } else if let Some(existing) = load_host_entry()? {
                if existing.is_fresh(config.update.host_ttl_seconds) {
                    existing
                } else {
                    refresh_host_cache(config)?
                }
            } else {
                refresh_host_cache(config)?
            };

            println!(
                "{}",
                serde_json::to_string_pretty(&entry)
                    .context("failed to serialize host cache update result")?
            );
        }
        UpdateScope::Project(project_args) => {
            let entry = refresh_or_load_project_cache(
                config,
                project_args.cwd.as_ref(),
                project_args.force,
            )?
            .ok_or_else(|| anyhow::anyhow!("failed to determine project context"))?;

            println!(
                "{}",
                serde_json::to_string_pretty(&entry)
                    .context("failed to serialize project cache update result")?
            );
        }
        UpdateScope::All(project_args) => {
            let host_entry = if project_args.force {
                refresh_host_cache(config)?
            } else if let Some(existing) = load_host_entry()? {
                if existing.is_fresh(config.update.host_ttl_seconds) {
                    existing
                } else {
                    refresh_host_cache(config)?
                }
            } else {
                refresh_host_cache(config)?
            };

            let project_entry = refresh_or_load_project_cache(
                config,
                project_args.cwd.as_ref(),
                project_args.force,
            )?;

            let payload = serde_json::json!({
                "host_cache": host_entry,
                "project_cache": project_entry
            });

            println!(
                "{}",
                serde_json::to_string_pretty(&payload)
                    .context("failed to serialize combined update result")?
            );
        }
    }

    Ok(())
}

fn handle_inspect(config: &Config, args: &InspectArgs) -> Result<()> {
    match &args.target {
        InspectTarget::Host(format_args) => inspect_host(config, format_args),
        InspectTarget::Project(project_args) => inspect_project(config, project_args),
        InspectTarget::Theme(theme_args) => inspect_theme(config, theme_args),
    }
}

fn inspect_host(config: &Config, args: &InspectFormatArgs) -> Result<()> {
    let search_dirs = default_theme_search_dirs();
    let host_cache = load_host_entry()?;

    let payload = serde_json::json!({
        "config_path_precedence": [
            "JETCTX_CONFIG override",
            "~/.config/jetctx/config.toml",
            "built-in defaults"
        ],
        "theme_search_precedence": search_dirs.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        "effective_theme": config.theme,
        "host_cache": host_cache
    });

    print_structured_json(&payload, args.format)
}

fn inspect_project(config: &Config, args: &InspectProjectArgs) -> Result<()> {
    let cwd = resolve_cwd(args.cwd.as_ref())?;
    let project_entry = load_or_refresh_project_cache(config, &cwd)?;
    let cache_path = project_entry
        .as_ref()
        .map(|entry| {
            cache::CachePaths::discover()
                .map(|paths| paths.project_file_for_root(&entry.root_path()))
                .ok()
        })
        .flatten();

    let payload = serde_json::json!({
        "cwd": cwd.display().to_string(),
        "cache_path": cache_path.map(|p| p.display().to_string()),
        "project_cache": project_entry
    });

    print_structured_json(&payload, args.format)
}

fn inspect_theme(config: &Config, args: &InspectThemeArgs) -> Result<()> {
    let theme = load_theme(config, args.theme.as_deref())?;
    let payload = serde_json::to_value(&theme).context("failed to serialize theme")?;
    print_structured_json(&payload, args.format)
}

fn handle_doctor(config: &Config) -> Result<()> {
    let search_dirs = default_theme_search_dirs();
    let resolved_theme = load_theme(config, None)?;
    let config_path = Config::default_config_path().map(|p| p.display().to_string());
    let host_cache = load_host_entry()?;

    let payload = serde_json::json!({
        "status": "ok",
        "config_theme": config.theme,
        "resolved_theme": resolved_theme.name,
        "theme_search_dirs": search_dirs.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        "config_resolution": {
            "primary_expected_path": "~/.config/jetctx/config.toml",
            "resolved_default_path": config_path,
            "fallback": "built-in defaults"
        },
        "host_cache_present": host_cache.is_some()
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&payload).context("failed to serialize doctor payload")?
    );

    Ok(())
}

fn load_theme(config: &Config, explicit_theme: Option<&str>) -> Result<Theme> {
    let search_dirs = default_theme_search_dirs();

    Theme::resolve(explicit_theme, Some(config.theme.as_str()), &search_dirs).with_context(|| {
        format!(
            "failed to resolve theme '{}' using search dirs: {}",
            explicit_theme.unwrap_or(config.theme.as_str()),
            search_dirs
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

fn print_output(
    rendered: &str,
    tmux_context: Option<&TmuxContext>,
    format: OutputFormat,
) -> Result<()> {
    match format {
        OutputFormat::Text => {
            println!("{rendered}");
        }
        OutputFormat::Json => {
            let payload = serde_json::json!({
                "rendered": rendered,
                "tmux_context": tmux_context
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&payload)
                    .context("failed to serialize renderer payload")?
            );
        }
    }

    Ok(())
}

fn print_structured_json(value: &serde_json::Value, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Text | OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(value)
                    .context("failed to serialize structured output")?
            );
        }
    }

    Ok(())
}

fn tmux_context(cwd: Option<&PathBuf>, host_cache: &cache::HostCacheEntry) -> TmuxContext {
    let _ = cwd;

    TmuxContext {
        battery_percent: host_cache.battery_percent,
        memory_used_bytes: host_cache.memory_used_bytes,
        memory_total_bytes: host_cache.memory_total_bytes,
        time_label: host_cache.time_label.clone(),
    }
}

fn resolve_cwd(cwd: Option<&PathBuf>) -> Result<PathBuf> {
    match cwd {
        Some(path) => Ok(path.clone()),
        None => std::env::current_dir().context("failed to determine current working directory"),
    }
}

fn refresh_or_load_project_cache(
    config: &Config,
    cwd: Option<&PathBuf>,
    force: bool,
) -> Result<Option<ProjectCacheEntry>> {
    let cwd = resolve_cwd(cwd)?;

    if force {
        return refresh_project_cache(&cwd, config);
    }

    load_or_refresh_project_entry(&cwd, config)
}

fn load_or_refresh_project_cache(
    config: &Config,
    cwd: &PathBuf,
) -> Result<Option<ProjectCacheEntry>> {
    load_or_refresh_project_entry(cwd, config)
}
