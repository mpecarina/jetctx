#!/usr/bin/env bash
# Create the isolated tmux session used by jetctx-tmux-demo.tape.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
socket_name="jetctx-terminal-rec"
session_name="jetctx-demo"
demo_root="${repo_root}/.terminal-rec/jetctx-tmux"
demo_repo="${demo_root}/api-control-plane"
demo_home="${demo_root}/home"
demo_config="${demo_root}/config.toml"

tmux_client() {
  tmux -L "${socket_name}" "$@"
}

cleanup() {
  tmux_client kill-server >/dev/null 2>&1 || true
}

setup() {
  cleanup
  rm -rf -- "${demo_root}"
  mkdir -p "${demo_repo}/src" "${demo_home}" "${demo_root}/cache"

  git init -q -b main "${demo_repo}"
  git -C "${demo_repo}" config user.name "Terminal Rec"
  git -C "${demo_repo}" config user.email "terminal-rec@example.invalid"
  printf '[package]\nname = "api-control-plane"\nversion = "0.1.0"\n' > "${demo_repo}/Cargo.toml"
  printf 'fn main() {}\n' > "${demo_repo}/src/main.rs"
  git -C "${demo_repo}" add Cargo.toml src/main.rs
  git -C "${demo_repo}" commit -qm "initial fixture"

  printf '%s\n' \
    'theme = "nightowl"' \
    '' \
    '[tmux]' \
    'show_memory = true' \
    'battery_symbol = "BAT"' \
    'memory_symbol = "MEM"' \
    'time_symbol = "TIME"' \
    > "${demo_config}"

  HOME="${demo_home}" \
  XDG_CACHE_HOME="${demo_root}/cache" \
  JETCTX_CONFIG="${demo_config}" \
  "${repo_root}/bin/jetctx" update host --force >/dev/null

  HOME="${demo_home}" \
  XDG_CACHE_HOME="${demo_root}/cache" \
  JETCTX_CONFIG="${demo_config}" \
  PATH="${repo_root}/bin:${PATH}" \
  tmux_client -f /dev/null new-session -d -s "${session_name}" -n shell \
    -x 132 -y 27 -c "${demo_repo}" \
    "env PS1='❯ ' bash --noprofile --norc"

  tmux_client set-environment -g JETCTX_CONFIG "${demo_config}"
  tmux_client set-environment -g XDG_CACHE_HOME "${demo_root}/cache"
  tmux_client set-environment -g PATH "${repo_root}/bin:${PATH}"
  tmux_client set-option -g automatic-rename off
  tmux_client set-option -g default-terminal tmux-256color
  tmux_client set-option -g mouse off
  tmux_client set-option -g status on
  tmux_client set-option -g status-interval 1
  tmux_client set-option -g status-style 'bg=#1e1e2e,fg=#cdd6f4'
  tmux_client set-option -g status-left '#[bold,fg=#89b4fa] api-control-plane #[default]'
  tmux_client set-option -g status-left-length 32
  tmux_client set-option -g window-status-current-format '#[fg=#cdd6f4] #I:#W '
  tmux_client set-option -g status-right-length 90
  tmux_client set-option -g status-right "#(${repo_root}/bin/jetctx tmux)"
  tmux_client send-keys -t "${session_name}:shell" clear Enter
}

case "${1:-}" in
  setup)
    setup
    ;;
  attach)
    exec tmux -L "${socket_name}" attach-session -t "${session_name}"
    ;;
  cleanup)
    cleanup
    ;;
  *)
    printf 'usage: %s setup|attach|cleanup\n' "$0" >&2
    exit 2
    ;;
esac
