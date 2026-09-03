#!/usr/bin/env bash
# The deterministic fixture behind docs/media/jetctx-demo.mp4.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
demo_parent="${repo_root}/.terminal-rec/jetctx-demo"
demo_repo="${demo_parent}/api-control-plane"

setup() {
  rm -rf -- "${demo_parent}"
  mkdir -p "${demo_repo}/src"
  git init -q -b main "${demo_repo}"
  git -C "${demo_repo}" config user.name "Terminal Rec"
  git -C "${demo_repo}" config user.email "terminal-rec@example.invalid"
  printf '[package]\nname = "api-control-plane"\nversion = "0.1.0"\n' > "${demo_repo}/Cargo.toml"
  printf 'fn main() {}\n' > "${demo_repo}/src/main.rs"
  git -C "${demo_repo}" add Cargo.toml src/main.rs
  git -C "${demo_repo}" commit -qm "initial fixture"
}

render_prompt() {
  local exit_code="$1"
  local duration_ms="${2:-}"
  local rendered
  local args=(prompt --cwd api-control-plane --exit-code "${exit_code}")

  if [[ -n "${duration_ms}" ]]; then
    args+=(--duration-ms "${duration_ms}")
  fi

  rendered="$(
    cd "${demo_parent}"
    export XDG_CACHE_HOME="${demo_parent}/cache"
    export JETCTX_CONFIG=/dev/null
    export JETCTX_THEME=nightowl
    "${repo_root}/bin/jetctx" update project --cwd api-control-plane --force >/dev/null
    "${repo_root}/bin/jetctx" "${args[@]}"
  )"
  zsh -f -c 'print -P -- "$1"' _ "${rendered}"
}

case "${1:-}" in
  setup)
    setup
    ;;
  clean)
    printf '\033[1;36mclean branch\033[0m\n'
    render_prompt 0
    ;;
  dirty)
    printf 'pub fn health() -> bool { true }\n' > "${demo_repo}/src/health.rs"
    printf '\033[1;33muncommitted change\033[0m\n'
    render_prompt 0
    ;;
  success)
    printf '\033[1;32msuccessful command with duration\033[0m\n'
    render_prompt 0 842
    ;;
  failure)
    printf '\033[1;31mstatus 1 with duration\033[0m\n'
    render_prompt 1 1274
    ;;
  *)
    printf 'usage: %s setup|clean|dirty|success|failure\n' "$0" >&2
    exit 2
    ;;
esac
