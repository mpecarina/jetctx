use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde::Serialize;

const PROJECT_MARKERS: &[ProjectMarker] = &[
    ProjectMarker::new(".git", ProjectKind::Git),
    ProjectMarker::new("Cargo.toml", ProjectKind::Rust),
    ProjectMarker::new("package.json", ProjectKind::Node),
    ProjectMarker::new("pyproject.toml", ProjectKind::Python),
    ProjectMarker::new("go.mod", ProjectKind::Go),
    ProjectMarker::new("Gemfile", ProjectKind::Ruby),
    ProjectMarker::new("flake.nix", ProjectKind::Nix),
    ProjectMarker::new(".terraform", ProjectKind::Terraform),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectKind {
    Plain,
    Git,
    Rust,
    Node,
    Python,
    Go,
    Ruby,
    Nix,
    Terraform,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectMarkerMatch {
    pub name: String,
    pub kind: ProjectKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectInfo {
    pub cwd: PathBuf,
    pub root: PathBuf,
    pub name: String,
    pub kind: ProjectKind,
    pub markers: Vec<ProjectMarkerMatch>,
}

#[derive(Debug, Clone, Copy)]
pub struct DetectOptions<'a> {
    pub stop_at: Option<&'a Path>,
    pub include_plain_fallback: bool,
}

impl<'a> Default for DetectOptions<'a> {
    fn default() -> Self {
        Self {
            stop_at: None,
            include_plain_fallback: true,
        }
    }
}

#[derive(Debug, Clone)]
struct ProjectMarker {
    name: &'static str,
    kind: ProjectKind,
}

impl ProjectMarker {
    const fn new(name: &'static str, kind: ProjectKind) -> Self {
        Self { name, kind }
    }
}

pub fn detect(cwd: &Path) -> Option<ProjectInfo> {
    detect_with_options(cwd, DetectOptions::default())
}

pub fn detect_with_options(cwd: &Path, options: DetectOptions<'_>) -> Option<ProjectInfo> {
    let start = normalize_start_dir(cwd)?;

    let mut current = start.as_path();
    loop {
        let matches = collect_marker_matches(current);

        if !matches.is_empty() {
            let kind = classify_kind(&matches);
            let name = project_name_for_root(current);

            return Some(ProjectInfo {
                cwd: start.clone(),
                root: current.to_path_buf(),
                name,
                kind,
                markers: matches,
            });
        }

        if should_stop(current, options.stop_at) {
            break;
        }

        let Some(parent) = current.parent() else {
            break;
        };

        if parent == current {
            break;
        }

        current = parent;
    }

    if options.include_plain_fallback {
        Some(ProjectInfo {
            cwd: start.clone(),
            root: start.clone(),
            name: project_name_for_root(&start),
            kind: ProjectKind::Plain,
            markers: Vec::new(),
        })
    } else {
        None
    }
}

fn normalize_start_dir(cwd: &Path) -> Option<PathBuf> {
    if cwd.is_dir() {
        Some(cwd.to_path_buf())
    } else {
        cwd.parent().map(Path::to_path_buf)
    }
}

fn collect_marker_matches(dir: &Path) -> Vec<ProjectMarkerMatch> {
    let mut matches = Vec::new();

    for marker in PROJECT_MARKERS {
        if dir.join(marker.name).exists() {
            matches.push(ProjectMarkerMatch {
                name: marker.name.to_string(),
                kind: marker.kind,
            });
        }
    }

    matches
}

fn classify_kind(matches: &[ProjectMarkerMatch]) -> ProjectKind {
    if let Some(kind) = matches
        .iter()
        .find(|marker| marker.kind != ProjectKind::Git)
        .map(|marker| marker.kind)
    {
        return kind;
    }

    if matches.iter().any(|marker| marker.kind == ProjectKind::Git) {
        return ProjectKind::Git;
    }

    ProjectKind::Plain
}

fn should_stop(current: &Path, stop_at: Option<&Path>) -> bool {
    match stop_at {
        Some(boundary) => current == boundary,
        None => false,
    }
}

fn project_name_for_root(root: &Path) -> String {
    root.file_name()
        .and_then(OsStr::to_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| root.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_to_plain_project_when_no_markers_exist() {
        let parent = std::env::temp_dir();
        let root = parent.join(format!("jetctx-project-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("temporary project should be created");

        let info = detect_with_options(
            &root,
            DetectOptions {
                stop_at: Some(&parent),
                include_plain_fallback: true,
            },
        )
        .expect("plain fallback should exist");

        assert_eq!(info.cwd, root);
        assert_eq!(info.root, root);
        assert_eq!(info.kind, ProjectKind::Plain);
        assert!(info.markers.is_empty());
        assert_eq!(
            info.name,
            format!("jetctx-project-test-{}", std::process::id())
        );

        std::fs::remove_dir_all(root).expect("temporary project should be removed");
    }

    #[test]
    fn no_plain_fallback_returns_none() {
        let root = PathBuf::from("/tmp/example");
        let info = detect_with_options(
            &root,
            DetectOptions {
                stop_at: Some(Path::new("/tmp")),
                include_plain_fallback: false,
            },
        );

        assert!(info.is_none());
    }

    #[test]
    fn classify_prefers_language_marker_over_git_marker() {
        let matches = vec![
            ProjectMarkerMatch {
                name: ".git".to_string(),
                kind: ProjectKind::Git,
            },
            ProjectMarkerMatch {
                name: "Cargo.toml".to_string(),
                kind: ProjectKind::Rust,
            },
        ];

        assert_eq!(classify_kind(&matches), ProjectKind::Rust);
    }

    #[test]
    fn classify_git_only_project() {
        let matches = vec![ProjectMarkerMatch {
            name: ".git".to_string(),
            kind: ProjectKind::Git,
        }];

        assert_eq!(classify_kind(&matches), ProjectKind::Git);
    }

    #[test]
    fn file_path_input_normalizes_to_parent_directory() {
        let file_path = PathBuf::from("/tmp/example/src/main.rs");
        let normalized = normalize_start_dir(&file_path).expect("parent directory should exist");
        assert_eq!(normalized, PathBuf::from("/tmp/example/src"));
    }
}
