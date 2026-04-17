use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::detect::{git, project};

#[derive(Debug, Clone)]
pub struct CachePaths {
    pub root: PathBuf,
    pub host_file: PathBuf,
    pub projects_dir: PathBuf,
}

impl CachePaths {
    pub fn discover() -> Result<Self> {
        let root = cache_root().context("failed to resolve jetctx cache root")?;
        let host_file = root.join("host.json");
        let projects_dir = root.join("projects");

        Ok(Self {
            root,
            host_file,
            projects_dir,
        })
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.root)
            .with_context(|| format!("failed to create cache root: {}", self.root.display()))?;
        fs::create_dir_all(&self.projects_dir).with_context(|| {
            format!(
                "failed to create project cache dir: {}",
                self.projects_dir.display()
            )
        })?;
        Ok(())
    }

    pub fn project_file_for_root(&self, root: &Path) -> PathBuf {
        self.projects_dir
            .join(format!("{}.json", stable_project_key(root)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectCacheEntry {
    pub version: u32,
    pub updated_at_epoch_secs: u64,
    pub cwd: String,
    pub root: String,
    pub project_name: String,
    pub project_kind: String,
    pub markers: Vec<ProjectMarkerCacheEntry>,
    pub git: Option<GitCacheEntry>,
}

impl ProjectCacheEntry {
    pub fn is_fresh(&self, ttl_seconds: u64) -> bool {
        is_fresh(self.updated_at_epoch_secs, ttl_seconds)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMarkerCacheEntry {
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCacheEntry {
    pub branch: Option<String>,
    pub dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostCacheEntry {
    pub version: u32,
    pub updated_at_epoch_secs: u64,
    pub theme: String,
    pub hostname: Option<String>,
    pub os: Option<String>,
    pub battery_percent: Option<u8>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    pub time_label: Option<String>,
}

impl HostCacheEntry {
    pub fn is_fresh(&self, ttl_seconds: u64) -> bool {
        is_fresh(self.updated_at_epoch_secs, ttl_seconds)
    }
}

pub fn load_project_entry(root: &Path) -> Result<Option<ProjectCacheEntry>> {
    let paths = CachePaths::discover()?;
    let file = paths.project_file_for_root(root);
    read_json_file::<ProjectCacheEntry>(&file)
}

pub fn load_project_entry_for_cwd(cwd: &Path) -> Result<Option<ProjectCacheEntry>> {
    let Some(project_info) = project::detect(cwd) else {
        return Ok(None);
    };

    load_project_entry(&project_info.root)
}

pub fn save_project_entry(entry: &ProjectCacheEntry) -> Result<PathBuf> {
    let paths = CachePaths::discover()?;
    paths.ensure_dirs()?;

    let file = paths.project_file_for_root(Path::new(&entry.root));
    write_json_file(&file, entry)?;
    Ok(file)
}

pub fn load_host_entry() -> Result<Option<HostCacheEntry>> {
    let paths = CachePaths::discover()?;
    read_json_file::<HostCacheEntry>(&paths.host_file)
}

pub fn save_host_entry(entry: &HostCacheEntry) -> Result<PathBuf> {
    let paths = CachePaths::discover()?;
    paths.ensure_dirs()?;
    write_json_file(&paths.host_file, entry)?;
    Ok(paths.host_file)
}

pub fn build_project_cache_entry(
    cwd: &Path,
    _config: &Config,
) -> Result<Option<ProjectCacheEntry>> {
    let Some(project_info) = project::detect(cwd) else {
        return Ok(None);
    };

    let git_ctx = if project_info.has_git_marker()
        || matches!(project_info.kind, project::ProjectKind::Git)
    {
        git::detect(&project_info.root)?
    } else {
        None
    };

    let markers = project_info
        .markers
        .iter()
        .map(|marker| ProjectMarkerCacheEntry {
            name: marker.name.clone(),
            kind: format!("{:?}", marker.kind).to_lowercase(),
        })
        .collect::<Vec<_>>();

    let git = git_ctx.map(|git_state| GitCacheEntry {
        branch: git_state.branch,
        dirty: git_state.dirty,
    });

    Ok(Some(ProjectCacheEntry {
        version: 1,
        updated_at_epoch_secs: now_epoch_secs(),
        cwd: cwd.display().to_string(),
        root: project_info.root.display().to_string(),
        project_name: project_info.name,
        project_kind: format!("{:?}", project_info.kind).to_lowercase(),
        markers,
        git,
    }))
}

pub fn build_host_cache_entry(config: &Config) -> HostCacheEntry {
    HostCacheEntry {
        version: 1,
        updated_at_epoch_secs: now_epoch_secs(),
        theme: config.theme.clone(),
        hostname: detect_hostname(),
        os: Some(std::env::consts::OS.to_string()),
        battery_percent: detect_battery_percent(),
        memory_used_bytes: detect_memory_used_bytes(),
        memory_total_bytes: detect_memory_total_bytes(),
        time_label: detect_time_label(),
    }
}

pub fn refresh_project_cache(cwd: &Path, config: &Config) -> Result<Option<ProjectCacheEntry>> {
    let Some(entry) = build_project_cache_entry(cwd, config)? else {
        return Ok(None);
    };

    save_project_entry(&entry)?;
    Ok(Some(entry))
}

pub fn load_or_refresh_project_entry(
    cwd: &Path,
    config: &Config,
) -> Result<Option<ProjectCacheEntry>> {
    if let Some(existing) = load_project_entry_for_cwd(cwd)? {
        if existing.is_fresh(config.update.project_ttl_seconds) {
            return Ok(Some(existing));
        }
    }

    refresh_project_cache(cwd, config)
}

pub fn refresh_host_cache(config: &Config) -> Result<HostCacheEntry> {
    let entry = build_host_cache_entry(config);
    save_host_entry(&entry)?;
    Ok(entry)
}

pub fn load_or_refresh_host_entry(config: &Config) -> Result<HostCacheEntry> {
    if let Some(existing) = load_host_entry()? {
        if existing.is_fresh(config.update.host_ttl_seconds) {
            return Ok(existing);
        }
    }

    refresh_host_cache(config)
}

fn detect_hostname() -> Option<String> {
    if let Ok(value) = std::env::var("HOSTNAME") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    command_stdout("hostname")
}

fn detect_battery_percent() -> Option<u8> {
    let output = command_stdout_with_args("pmset", &["-g", "batt"])?;
    let percent = output
        .split_whitespace()
        .find(|part| part.contains('%'))?
        .trim_end_matches(|c: char| c == '%' || c == ';')
        .parse::<u8>()
        .ok()?;
    Some(percent)
}

fn detect_memory_total_bytes() -> Option<u64> {
    command_stdout_with_args("sysctl", &["-n", "hw.memsize"])?
        .trim()
        .parse::<u64>()
        .ok()
}

fn detect_memory_used_bytes() -> Option<u64> {
    let output = command_stdout("vm_stat")?;
    let page_size = output.lines().next().and_then(parse_vm_stat_page_size)?;

    let mut pages_free = 0_u64;
    let mut pages_speculative = 0_u64;

    for line in output.lines().skip(1) {
        if let Some(value) = parse_vm_stat_counter(line, "Pages free") {
            pages_free = value;
        } else if let Some(value) = parse_vm_stat_counter(line, "Pages speculative") {
            pages_speculative = value;
        }
    }

    let total = detect_memory_total_bytes()?;
    let free_bytes = (pages_free + pages_speculative).saturating_mul(page_size);
    Some(total.saturating_sub(free_bytes))
}

fn detect_time_label() -> Option<String> {
    command_stdout_with_args("date", &["+%H:%M"])
}

fn command_stdout(cmd: &str) -> Option<String> {
    command_stdout_with_args(cmd, &[])
}

fn command_stdout_with_args(cmd: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(cmd).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn parse_vm_stat_page_size(line: &str) -> Option<u64> {
    let start = line.find("page size of ")? + "page size of ".len();
    let end = line[start..].find(" bytes")? + start;
    line[start..end].trim().parse::<u64>().ok()
}

fn parse_vm_stat_counter(line: &str, label: &str) -> Option<u64> {
    let value = line.strip_prefix(label)?.trim();
    let value = value.strip_prefix(':')?.trim().trim_end_matches('.');
    value.parse::<u64>().ok()
}

fn cache_root() -> Result<PathBuf> {
    if let Ok(xdg_cache_home) = std::env::var("XDG_CACHE_HOME") {
        let trimmed = xdg_cache_home.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed).join("jetctx"));
        }
    }

    let home = dirs::home_dir().context("home directory is unavailable")?;
    Ok(home.join(".cache").join("jetctx"))
}

fn stable_project_key(path: &Path) -> String {
    let canonicalish = path.to_string_lossy();
    let mut hasher = DefaultHasher::new();
    canonicalish.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn read_json_file<T>(path: &Path) -> Result<Option<T>>
where
    T: for<'de> Deserialize<'de>,
{
    match fs::read_to_string(path) {
        Ok(contents) => {
            let value = serde_json::from_str::<T>(&contents)
                .with_context(|| format!("failed to parse JSON cache file: {}", path.display()))?;
            Ok(Some(value))
        }
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => {
            Err(err).with_context(|| format!("failed to read cache file: {}", path.display()))
        }
    }
}

fn write_json_file<T>(path: &Path, value: &T) -> Result<()>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent directory: {}", parent.display()))?;
    }

    let payload =
        serde_json::to_string_pretty(value).context("failed to serialize cache payload")?;
    fs::write(path, payload)
        .with_context(|| format!("failed to write cache file: {}", path.display()))
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn is_fresh(updated_at_epoch_secs: u64, ttl_seconds: u64) -> bool {
    if ttl_seconds == 0 {
        return false;
    }

    let now = now_epoch_secs();
    now.saturating_sub(updated_at_epoch_secs) <= ttl_seconds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freshness_check_respects_ttl() {
        let now = now_epoch_secs();
        assert!(is_fresh(now, 30));
        assert!(!is_fresh(now.saturating_sub(31), 30));
    }

    #[test]
    fn stable_project_key_is_deterministic() {
        let path = Path::new("/tmp/example");
        let a = stable_project_key(path);
        let b = stable_project_key(path);
        assert_eq!(a, b);
    }

    #[test]
    fn project_cache_entry_root_round_trips() {
        let entry = ProjectCacheEntry {
            version: 1,
            updated_at_epoch_secs: 1,
            cwd: "/tmp/example".to_string(),
            root: "/tmp/example".to_string(),
            project_name: "example".to_string(),
            project_kind: "plain".to_string(),
            markers: Vec::new(),
            git: None,
        };

        assert_eq!(entry.root_path(), PathBuf::from("/tmp/example"));
    }
}
