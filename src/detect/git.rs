use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use serde::Serialize;

/// Minimal git summary for prompt/tmux rendering.
///
/// Design notes:
/// - Keep this implementation simple and reliable first.
/// - Prefer cheap local inspection where practical (`.git`, `HEAD`).
/// - Allow one bounded `git status --porcelain --branch` invocation for a
///   first-pass dirty/ahead/behind summary.
/// - This module is intentionally cache-friendly: the returned summary includes
///   a few mtimes that higher layers can later use for invalidation.
#[derive(Debug, Clone, Serialize)]
pub struct GitContext {
    pub repo_root: PathBuf,
    pub git_dir: PathBuf,
    pub branch: Option<String>,
    pub head_oid_short: Option<String>,
    pub dirty: bool,
    pub staged: usize,
    pub modified: usize,
    pub untracked: usize,
    pub conflicted: usize,
    pub ahead: usize,
    pub behind: usize,
    pub detached: bool,
    pub head_mtime: Option<u64>,
    pub index_mtime: Option<u64>,
}

/// Detect git context for a working directory.
///
/// Returns `Ok(None)` when the path is not inside a git repository.
pub fn detect(cwd: &Path) -> Result<Option<GitContext>> {
    let Some(repo_root) = find_repo_root(cwd)? else {
        return Ok(None);
    };

    let git_dir = resolve_git_dir(&repo_root)?;
    let head_mtime = file_mtime_secs(&git_dir.join("HEAD"));
    let index_mtime = file_mtime_secs(&git_dir.join("index"));

    let branch = read_branch_from_head(&git_dir)?;
    let detached = branch.is_none();

    let head_oid_short = read_head_oid_short(&repo_root).ok().flatten();

    let status = collect_status(&repo_root).unwrap_or_default();

    Ok(Some(GitContext {
        repo_root,
        git_dir,
        branch,
        head_oid_short,
        dirty: status.dirty,
        staged: status.staged,
        modified: status.modified,
        untracked: status.untracked,
        conflicted: status.conflicted,
        ahead: status.ahead,
        behind: status.behind,
        detached,
        head_mtime,
        index_mtime,
    }))
}

/// Find the git repository root by walking upward from `start`.
///
/// This intentionally recognizes only `.git` markers for now.
pub fn find_repo_root(start: &Path) -> Result<Option<PathBuf>> {
    let mut current = if start.is_dir() {
        start.to_path_buf()
    } else {
        start
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| start.to_path_buf())
    };

    loop {
        let marker = current.join(".git");
        if marker.exists() {
            return Ok(Some(current));
        }

        let Some(parent) = current.parent() else {
            return Ok(None);
        };

        if parent == current {
            return Ok(None);
        }

        current = parent.to_path_buf();
    }
}

/// Resolve the actual git directory for a repo root.
///
/// Handles:
/// - normal `.git/` directory
/// - gitfile indirection: `.git` file containing `gitdir: ...`
pub fn resolve_git_dir(repo_root: &Path) -> Result<PathBuf> {
    let dot_git = repo_root.join(".git");

    if dot_git.is_dir() {
        return Ok(dot_git);
    }

    if dot_git.is_file() {
        let raw = fs::read_to_string(&dot_git)
            .with_context(|| format!("failed reading gitfile: {}", dot_git.display()))?;
        let line = raw.trim();

        let prefix = "gitdir:";
        if let Some(rest) = line.strip_prefix(prefix) {
            let value = rest.trim();
            let candidate = PathBuf::from(value);
            if candidate.is_absolute() {
                return Ok(candidate);
            }
            return Ok(repo_root.join(candidate));
        }

        anyhow::bail!("unsupported gitfile format: {}", dot_git.display());
    }

    anyhow::bail!("missing .git marker under {}", repo_root.display());
}

fn read_branch_from_head(git_dir: &Path) -> Result<Option<String>> {
    let head = fs::read_to_string(git_dir.join("HEAD"))
        .with_context(|| format!("failed reading HEAD in {}", git_dir.display()))?;

    let head = head.trim();

    if let Some(reference) = head.strip_prefix("ref:") {
        return Ok(branch_name_from_reference(reference.trim()));
    }

    Ok(None)
}

fn branch_name_from_reference(reference: &str) -> Option<String> {
    let branch = reference
        .strip_prefix("refs/heads/")
        .unwrap_or(reference)
        .trim();

    if branch.is_empty() {
        None
    } else {
        Some(branch.to_string())
    }
}

fn read_head_oid_short(repo_root: &Path) -> Result<Option<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("rev-parse")
        .arg("--short")
        .arg("HEAD")
        .output()
        .with_context(|| format!("failed to execute git rev-parse in {}", repo_root.display()))?;

    if !output.status.success() {
        return Ok(None);
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}

#[derive(Debug, Clone, Default)]
struct GitStatusCounters {
    dirty: bool,
    staged: usize,
    modified: usize,
    untracked: usize,
    conflicted: usize,
    ahead: usize,
    behind: usize,
}

fn collect_status(repo_root: &Path) -> Result<GitStatusCounters> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("status")
        .arg("--porcelain")
        .arg("--branch")
        .output()
        .with_context(|| format!("failed to execute git status in {}", repo_root.display()))?;

    if !output.status.success() {
        return Ok(GitStatusCounters::default());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut counters = GitStatusCounters::default();

    for line in stdout.lines() {
        if line.starts_with("## ") {
            parse_branch_header(line, &mut counters);
            continue;
        }

        if line.len() < 2 {
            continue;
        }

        let x = line.as_bytes()[0] as char;
        let y = line.as_bytes()[1] as char;

        if line.starts_with("??") {
            counters.untracked += 1;
            counters.dirty = true;
            continue;
        }

        if is_conflict_pair(x, y) {
            counters.conflicted += 1;
            counters.dirty = true;
            continue;
        }

        if x != ' ' {
            counters.staged += 1;
            counters.dirty = true;
        }

        if y != ' ' {
            counters.modified += 1;
            counters.dirty = true;
        }
    }

    Ok(counters)
}

fn parse_branch_header(line: &str, counters: &mut GitStatusCounters) {
    // Examples:
    // ## main
    // ## feature...origin/feature [ahead 2]
    // ## feature...origin/feature [behind 1]
    // ## feature...origin/feature [ahead 2, behind 1]
    if let Some(start) = line.find('[') {
        if let Some(end) = line.rfind(']') {
            if end > start {
                let details = &line[start + 1..end];
                for part in details.split(',') {
                    let part = part.trim();
                    if let Some(value) = part.strip_prefix("ahead ") {
                        if let Ok(n) = value.trim().parse::<usize>() {
                            counters.ahead = n;
                        }
                    } else if let Some(value) = part.strip_prefix("behind ") {
                        if let Ok(n) = value.trim().parse::<usize>() {
                            counters.behind = n;
                        }
                    }
                }
            }
        }
    }
}

fn is_conflict_pair(x: char, y: char) -> bool {
    matches!(
        (x, y),
        ('D', 'D') | ('A', 'U') | ('U', 'D') | ('U', 'A') | ('D', 'U') | ('A', 'A') | ('U', 'U')
    )
}

fn file_mtime_secs(path: &Path) -> Option<u64> {
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let duration = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_hierarchical_branch_names() {
        assert_eq!(
            branch_name_from_reference("refs/heads/feature/deep/name"),
            Some("feature/deep/name".to_string())
        );
    }

    #[test]
    fn parses_ahead_behind_header() {
        let mut counters = GitStatusCounters::default();
        parse_branch_header("## feat...origin/feat [ahead 2, behind 1]", &mut counters);

        assert_eq!(counters.ahead, 2);
        assert_eq!(counters.behind, 1);
    }

    #[test]
    fn parses_ahead_only_header() {
        let mut counters = GitStatusCounters::default();
        parse_branch_header("## feat...origin/feat [ahead 4]", &mut counters);

        assert_eq!(counters.ahead, 4);
        assert_eq!(counters.behind, 0);
    }

    #[test]
    fn detects_conflict_pairs() {
        assert!(is_conflict_pair('U', 'U'));
        assert!(is_conflict_pair('A', 'U'));
        assert!(!is_conflict_pair('M', ' '));
        assert!(!is_conflict_pair(' ', 'M'));
    }
}
