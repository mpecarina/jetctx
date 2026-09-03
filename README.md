# jetctx

shared prompt and tmux context renderer.

`jetctx` renders:

- a zsh prompt with cwd, git branch/dirty state, and optional command duration
- a tmux right-side status segment with battery, memory, and time

It uses one config file, one theme, and shared cache files under `~/.cache/jetctx/`.

## Demo

https://github.com/user-attachments/assets/a1787cd5-cfc2-4b70-b783-0a160388fb12

## Tmux

```tmux
set -g status-right '#(~/.tmux/plugins/jetctx/bin/jetctx tmux)'
```

https://github.com/user-attachments/assets/562adb16-1e68-46f3-b810-81d22de92a17

## Install (TPM)

Add this to your `~/.tmux.conf`:

```tmux
set -g @plugin 'mpecarina/jetctx'

# tmux status-right
set -g status-right '#(~/.tmux/plugins/jetctx/bin/jetctx tmux)'

run '~/.tmux/plugins/tpm/tpm'
```

Then in tmux: `prefix + I` to install. A Rust toolchain with `cargo` is required
for the first build.

`bin/jetctx` is a tracked wrapper that auto-builds `target/release/jetctx` when the
binary is missing, the git commit changed, or local Rust sources are newer.

## Install (Manual)

```sh
cargo build --release --locked
install -m 755 target/release/jetctx "$HOME/.local/bin/jetctx"
mkdir -p "$HOME/.config/jetctx/themes"
for theme in themes/*.toml; do
  destination="$HOME/.config/jetctx/themes/${theme##*/}"
  test -e "$destination" || cp "$theme" "$destination"
done
```

Then use `~/.local/bin/jetctx` in your shell prompt and tmux config. Existing
user-managed themes are not overwritten.

## Zsh Prompt

The prompt output uses zsh prompt color escapes, so wire it into zsh rather than a
generic POSIX prompt.

Minimal `~/.zshrc` example:

```zsh
setopt prompt_subst
export PATH="$PATH:$HOME/.tmux/plugins/jetctx/bin"

jetctx_precmd() {
  local exit_code=$?
  PROMPT="$(jetctx prompt --cwd "$PWD" --exit-code "$exit_code") "
}

precmd_functions+=(jetctx_precmd)
```

If you already track command timing in your shell, pass it through with
`--duration-ms <ms>`.

## Config

Config resolution order:

- `JETCTX_CONFIG`
- `~/.config/jetctx/config.toml`
- built-in defaults

Example `~/.config/jetctx/config.toml`:

```toml
theme = "nightowl"

[prompt]
show_git = true
show_duration = true
duration_min_ms = 400

[tmux]
show_memory = true
battery_symbol = "BAT"
memory_symbol = "MEM"
time_symbol = "◷"

[update]
host_ttl_seconds = 15
project_ttl_seconds = 3
```

Theme override:

```sh
export JETCTX_THEME=shaman
```

Bundled themes:

- `nightowl`
- `shaman`
- `flat`

Theme search order currently prefers `~/.config/jetctx/themes/` before bundled repo
themes.

## Commands

```sh
jetctx prompt --cwd "$PWD" --exit-code 0
jetctx prompt --cwd "$PWD" --exit-code 1 --duration-ms 842
jetctx tmux
jetctx update host --force
jetctx update project --cwd "$PWD"
jetctx inspect host
jetctx inspect project --cwd "$PWD"
jetctx inspect theme
jetctx doctor
```

## Notes

- Prompt rendering is currently zsh-oriented.
- Host cache collection is currently macOS-oriented (`pmset`, `vm_stat`, `sysctl`, `date`).
- Project cache updates are consumed by prompt rendering rather than tmux.
