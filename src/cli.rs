use clap::{Args, Parser, Subcommand, ValueEnum};
use std::env;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "jetctx",
    version,
    about = "Unified terminal context renderer for prompt and tmux",
    long_about = "jetctx renders fast shell prompt and tmux status context from a shared model."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn parse_from_env() -> Self {
        Self::parse_from(env::args_os())
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Render prompt context
    Prompt(PromptArgs),

    /// Render tmux status content
    Tmux(TmuxArgs),

    /// Update cached host or project state
    Update(UpdateArgs),

    /// Inspect cached state or effective theme
    Inspect(InspectArgs),

    /// Run lightweight diagnostics
    Doctor,

    /// Print version information
    Version,
}

#[derive(Debug, Clone, Args)]
pub struct PromptArgs {
    /// Working directory to render context for
    #[arg(long)]
    pub cwd: Option<PathBuf>,

    /// Exit code of the previous command
    #[arg(long, default_value_t = 0)]
    pub exit_code: i32,

    /// Duration of the previous command in milliseconds
    #[arg(long)]
    pub duration_ms: Option<u64>,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct TmuxArgs {
    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct UpdateArgs {
    #[command(subcommand)]
    pub scope: UpdateScope,
}

#[derive(Debug, Clone, Subcommand)]
pub enum UpdateScope {
    /// Update host/system cache
    Host(UpdateTargetArgs),

    /// Update project cache for a working directory
    Project(ProjectTargetArgs),

    /// Update both host and project state
    All(ProjectTargetArgs),
}

#[derive(Debug, Clone, Args, Default)]
pub struct UpdateTargetArgs {
    /// Force refresh even if cache appears fresh
    #[arg(long, default_value_t = false)]
    pub force: bool,
}

#[derive(Debug, Clone, Args, Default)]
pub struct ProjectTargetArgs {
    /// Working directory to resolve project context from
    #[arg(long)]
    pub cwd: Option<PathBuf>,

    /// Force refresh even if cache appears fresh
    #[arg(long, default_value_t = false)]
    pub force: bool,
}

#[derive(Debug, Clone, Args)]
pub struct InspectArgs {
    #[command(subcommand)]
    pub target: InspectTarget,
}

#[derive(Debug, Clone, Subcommand)]
pub enum InspectTarget {
    /// Inspect cached host/system state
    Host(InspectFormatArgs),

    /// Inspect effective theme data
    Theme(InspectThemeArgs),
}

#[derive(Debug, Clone, Args)]
pub struct InspectFormatArgs {
    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct InspectThemeArgs {
    /// Theme name override
    #[arg(long)]
    pub theme: Option<String>,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Json => write!(f, "json"),
        }
    }
}
