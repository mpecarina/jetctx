# RFC 0001: `jetctx` Unified Prompt and Tmux Context Engine

- Status: Draft
- Author: `mpecarina` + AI-assisted draft
- Created: 2026-04-16
- Target Version: `v0.1.0`

## 1. Summary

`jetctx` is a small local terminal context engine that renders:

- a fast shell prompt
- a fast tmux status bar

from a shared context model and cache.

The primary goal is to replace:

- a heavy shell prompt configuration that mixes command-local and ambient system information
- a tmux status system that shells out repeatedly for expensive segments

with a single, coordinated, low-latency implementation.

`jetctx` is intended to be implemented as a compiled binary, preferably in Rust.

## 2. Motivation

The current terminal UX is split between:

1. a shell prompt system that exposes project, git, runtime, and some system information
2. a tmux status bar that exposes session and system information

This leads to several problems:

- duplicate work across prompt and tmux
- expensive repeated shell-outs
- too much information in the prompt
- no shared cache or ownership model for context data
- inconsistent theming and segment responsibilities

### 2.1 Problems to Solve

#### Prompt problems

The prompt currently attempts to show too much:

- directory
- git branch/status
- command duration
- many language/runtime modules
- battery/time
- host/local IP
- cloud/toolchain context

This is visually polished but expensive and conceptually overloaded.

#### tmux problems

The tmux status system currently:

- shells out for many segments
- refreshes frequently
- performs some expensive checks inline
- uses multiple independent scripts
- has some slow segments that should be cached or background-collected

### 2.2 Desired Outcome

We want:

- one fast binary
- one shared context model
- one shared cache
- one theme system
- one clear ownership contract between prompt and tmux

## 3. Goals

### 3.1 Primary Goals

`jetctx` MUST:

- render a shell prompt quickly
- render a tmux status bar quickly
- share state between prompt and tmux
- avoid duplicate expensive work
- avoid expensive probes during prompt rendering
- support at least two themes:
  - `nightowl`
  - `shaman`
- degrade gracefully when data is missing or stale

### 3.2 Secondary Goals

`jetctx` SHOULD:

- support both text and JSON output
- support platform-specific host/system data collection
- support configurable segment enablement
- support cache invalidation via TTL and filesystem mtimes
- be easy to inspect/debug

### 3.3 Non-Goals for v1

`jetctx` v1 will NOT attempt to:

- replace every feature of Starship
- implement dozens of language modules
- perform network/API calls during interactive render
- provide a plugin ecosystem
- manage tmux sessions/workspaces
- be a generic terminal UI framework

## 4. Core Design Principles

### 4.1 Prompt is for command-local context

The shell prompt SHOULD show only information that matters immediately before the next command is run.

Examples:

- cwd
- project/repo identity
- git summary
- active environment
- one relevant runtime
- last command status
- command duration
- ssh/root indicators

The prompt SHOULD NOT show ambient telemetry such as:

- battery
- memory
- CPU
- weather
- clock
- broad host metadata unless conditional

### 4.2 tmux is for ambient session/system context

The tmux status bar SHOULD show stable, session-level, and machine-level information.

Examples:

- session name
- window name/index
- active pane project summary
- battery
- CPU
- memory
- network summary
- time/date
- optional weather

The tmux status bar SHOULD NOT perform expensive project analysis on every refresh.

### 4.3 Compute once, reuse everywhere

If project/git state is computed for prompt rendering, tmux SHOULD be able to reuse it.

If host telemetry is collected for tmux, prompt SHOULD NOT attempt to duplicate it.

### 4.4 Interactive render paths must stay cheap

Prompt rendering MUST NOT block on:

- network
- weather
- broad runtime probing
- expensive recursive scans
- repeated heavy shell-outs

tmux status rendering SHOULD also avoid these operations inline.

### 4.5 Themes should be data, not code

Theme palette data SHOULD be stored separately from rendering logic.

## 5. Terminology

### Prompt
The shell-rendered, per-command interactive context line.

### tmux status bar
The tmux-rendered session/window/system context display.

### Host context
Machine-wide state such as battery, CPU, memory, network, weather.

### Project context
Directory/project-specific state such as project root, git branch, dirty state, runtime markers.

### Invocation context
Ephemeral shell-local values such as current `PWD`, exit status, and command duration.

## 6. High-Level Architecture

`jetctx` consists of four conceptual parts:

1. binary CLI
2. shared cache
3. detectors/collectors
4. renderers

### 6.1 Binary CLI

A single executable named `jetctx` will expose subcommands for:

- prompt rendering
- tmux rendering
- cache updates
- diagnostics and inspection

### 6.2 Shared Cache

Cache files will live under:

- `~/.cache/jetctx/`

They will store:

- host context
- project context
- optional runtime metadata

### 6.3 Detectors/Collectors

Collection code will gather:

- project root
- git state
- runtime markers and versions
- host system information

### 6.4 Renderers

Renderers will convert context into:

- prompt strings
- tmux status strings
- optional JSON payloads

## 7. Ownership Model

### 7.1 Prompt-Owned Information

The prompt SHOULD own:

- working directory
- project root / project name
- git branch
- git minimal dirty/staged/untracked summary
- active environment
- one dominant runtime if cheap
- previous exit code
- command duration
- root/ssh indication
- prompt character

### 7.2 tmux-Owned Information

tmux SHOULD own:

- session name
- window index/name
- host badge
- active pane project summary
- battery
- CPU
- memory
- network
- time/date
- optional weather

### 7.3 Shared Information

Both prompt and tmux MAY consume shared cached data for:

- project root mapping
- git state
- dominant runtime
- theme selection
- hostname

## 8. CLI Specification

## 8.1 Prompt Commands

```text
jetctx prompt
jetctx prompt --cwd /path
jetctx prompt --cwd /path --exit-code 1 --duration-ms 842
jetctx prompt --format text
jetctx prompt --format json
```

### Prompt Input Parameters

Prompt rendering MAY consume:

- `--cwd`
- `--exit-code`
- `--duration-ms`
- optional shell/env metadata in future versions

## 8.2 tmux Commands

```text
jetctx tmux left
jetctx tmux right
jetctx tmux render
jetctx tmux active-pane --pane-id %3
jetctx tmux right --format json
```

### tmux Rendering Strategy

v1 SHOULD support either:

- separate `left` and `right` rendering
- or a single `render` mode

Supporting both is acceptable.

## 8.3 Update Commands

```text
jetctx update host
jetctx update project --cwd /path
jetctx update all --cwd /path
jetctx refresh host
jetctx refresh project --cwd /path
```

### Notes

- `update` MAY respect cache freshness and TTL
- `refresh` SHOULD force recollection

## 8.4 Diagnostics Commands

```text
jetctx doctor
jetctx inspect host
jetctx inspect project --cwd /path
jetctx inspect theme
jetctx version
```

These commands exist to make failures and state visible outside interactive UI rendering.

## 9. Output Formats

### 9.1 Text

Default mode SHOULD be plain text suitable for direct shell/tmux consumption.

### 9.2 JSON

Optional mode SHOULD provide structured output for:

- debugging
- tests
- future integrations

## 10. Cache Layout

### 10.1 Root Directory

```text
~/.cache/jetctx/
  host.json
  host.lock
  projects/
  runtime/
```

### 10.2 Host Cache

```text
~/.cache/jetctx/host.json
```

### 10.3 Project Cache

Project cache SHOULD be keyed by canonical project root hash:

```text
~/.cache/jetctx/projects/<hash>.json
```

### 10.4 Runtime Cache

Optional runtime-specific cache MAY be stored separately:

```text
~/.cache/jetctx/runtime/<hash>.json
```

## 11. Host Cache Schema

Suggested first-pass schema:

```json
{
  "version": 1,
  "updated_at": "2026-04-16T18:00:00Z",
  "hostname": "LIBP45P-193439J",
  "os": "macos",
  "kernel": "Darwin",
  "network": {
    "online": true,
    "transport": "wifi",
    "ssid": "example",
    "updated_at": "2026-04-16T17:59:50Z"
  },
  "battery": {
    "percent": 82,
    "charging": false,
    "power_source": "battery",
    "updated_at": "2026-04-16T17:59:45Z"
  },
  "cpu": {
    "usage_percent": 8.4,
    "load_1": 1.22,
    "updated_at": "2026-04-16T17:59:58Z"
  },
  "memory": {
    "used_bytes": 17179869184,
    "total_bytes": 34359738368,
    "updated_at": "2026-04-16T17:59:58Z"
  },
  "weather": {
    "enabled": false,
    "text": null,
    "temperature": null,
    "icon": null,
    "updated_at": null,
    "expires_at": null
  },
  "theme": "nightowl"
}
```

## 12. Project Cache Schema

Suggested first-pass schema:

```json
{
  "version": 1,
  "root": "/Users/n1578295/.config/dotfiles",
  "project_name": "dotfiles",
  "kind": "git",
  "updated_at": "2026-04-16T18:00:00Z",
  "markers": {
    "git": true,
    "cargo_toml": false,
    "package_json": false,
    "pyproject_toml": false,
    "go_mod": false
  },
  "git": {
    "branch": "main",
    "head_oid_short": "abc1234",
    "dirty": true,
    "staged": 1,
    "modified": 2,
    "untracked": 0,
    "conflicted": 0,
    "stashed": 0,
    "ahead": 0,
    "behind": 0,
    "head_mtime": 1713280000,
    "index_mtime": 1713280012,
    "updated_at": "2026-04-16T18:00:00Z"
  },
  "runtime": {
    "active_env": null,
    "dominant": "lua",
    "python": null,
    "node": null,
    "rust": null,
    "go": null,
    "updated_at": "2026-04-16T17:59:59Z"
  }
}
```

## 13. Theme Specification

### 13.1 Theme Goals

A theme MUST be usable by both:

- prompt renderer
- tmux renderer

without embedding renderer-specific assumptions into core palette definitions.

### 13.2 Theme Location

Recommended locations:

- `~/.config/jetctx/themes/nightowl.toml`
- `~/.config/jetctx/themes/shaman.toml`

`jetctx` MUST treat the XDG/user config location as the primary theme source.

Project-local bundled themes MAY exist for development and fallback purposes, but they MUST NOT take precedence over user-managed config under `~/.config/jetctx/themes/`.

### 13.3 Theme Format

Themes SHOULD use TOML.

Example:

```toml
name = "nightowl"
kind = "dark"

[base]
bg = "#011627"
fg = "#d6deeb"
muted = "#637777"
surface = "#0b2239"
border = "#44596b"

[accent]
blue = "#82aaff"
cyan = "#7fdbca"
green = "#22da6e"
yellow = "#ffeb95"
orange = "#f78c6c"
red = "#ef5350"
purple = "#c792ea"

[semantic]
ok = "#22da6e"
warn = "#ffeb95"
error = "#ef5350"
info = "#82aaff"
```

### 13.4 Theme Key Rules

Themes SHOULD prefer semantic and structural keys such as:

- `bg`
- `fg`
- `surface`
- `border`
- `ok`
- `warn`
- `error`
- `info`

instead of prompt-specific or tmux-specific slot names.

### 13.5 Theme Precedence

Theme selection SHOULD follow this precedence:

1. CLI override
2. environment override
3. config file
4. default

After a theme name is resolved, theme file lookup MUST follow this precedence:

1. `~/.config/jetctx/themes/<name>.toml`
2. project-local bundled themes such as `<project-root>/themes/<name>.toml`

If both exist, the user-managed config theme MUST win.

## 14. Config File Specification

### 14.1 Config Location

```text
~/.config/jetctx/config.toml
```

### 14.2 Suggested Initial Shape

```toml
theme = "nightowl"

[prompt]
show_git = true
show_duration = true
duration_min_ms = 400
show_jobs = true
show_env = true
show_runtime = true
runtime_mode = "dominant"
show_user = "conditional"
show_host = "ssh"
show_sudo = true

[tmux]
show_session = true
show_window = true
show_project = true
show_git = true
show_battery = true
show_cpu = true
show_memory = true
show_network = true
show_time = true
show_weather = false

[update]
host_ttl_seconds = 15
cpu_ttl_seconds = 5
memory_ttl_seconds = 5
network_ttl_seconds = 15
battery_ttl_seconds = 20
weather_ttl_seconds = 1800
project_ttl_seconds = 3
runtime_ttl_seconds = 30

[git]
ignore_branches = ["main", "master"]
show_ahead_behind = true
show_stash = false

[weather]
enabled = false
zip = ""
country = "US"
label = ""
```

## 15. Detection Rules

### 15.1 Project Root Detection

Project detection SHOULD walk upward from `cwd`, checking a small set of markers:

- `.git`
- `Cargo.toml`
- `package.json`
- `pyproject.toml`
- `go.mod`
- `Gemfile`
- `flake.nix`
- `.terraform`

Detection SHOULD stop at:

- filesystem root
- optionally a configured boundary such as home directory

### 15.2 Git Detection

#### Fast Path

Git detection SHOULD prefer:

- direct `.git` resolution
- `HEAD` reads
- `.git/index` mtimes
- merge/rebase state marker files

#### Refresh Path

When mtimes change or cache expires, `jetctx` MAY run one bounded git command to refresh summary state.

`jetctx` SHOULD avoid multiple git subprocesses when one summary command can provide equivalent information.

### 15.3 Runtime Detection

v1 SHOULD support a small runtime set only:

- Python
- Node
- Rust
- Go

Detection SHOULD use:

- marker files first
- active shell environment second
- interpreter/tool invocations only when relevant and cacheable

## 16. Segment Contract

## 16.1 Prompt Segment Order

Recommended order:

1. status
2. user/host (conditional)
3. directory
4. git
5. active environment
6. runtime
7. duration
8. prompt character

### Prompt Display Rules

- empty segments MUST be omitted
- duration SHOULD only render above a threshold
- user/host SHOULD only render conditionally
- runtime SHOULD be concise

## 16.2 tmux Segment Layout

### Left

Recommended left segments:

1. session
2. window index/name

### Right

Recommended right segments:

1. active project
2. git branch summary
3. network
4. battery
5. CPU
6. memory
7. weather (optional)
8. time/date

### tmux Display Rules

- active-pane project context SHOULD be preferred over broad global project state
- git information in tmux SHOULD remain concise
- expensive system collection MUST NOT occur inline in the renderer

## 17. Performance Requirements

### 17.1 Prompt

Common cached prompt renders SHOULD target:

- under 5 ms in normal cases
- under 15 ms in typical git repo cases

Prompt rendering MUST NOT block on network.

### 17.2 tmux

tmux status rendering SHOULD target:

- under 10 ms using cached data

### 17.3 Collectors

Collectors MAY be slower than renderers, but SHOULD be bounded and separate from interactive paths.

## 18. Update Policy

### 18.1 Prompt Path

On prompt invocation, `jetctx` SHOULD:

1. gather invocation context
2. resolve project root
3. load cache
4. render immediately if cache is fresh
5. refresh minimally if stale and cheap

### 18.2 tmux Path

On tmux render, `jetctx` SHOULD:

1. read host cache
2. read project cache as needed
3. render immediately
4. avoid inline expensive collection

### 18.3 Host Refresh Cadence

Suggested defaults:

- battery: 15–30s
- CPU: 2–5s
- memory: 5–10s
- network: 10–30s
- weather: 15–60m

### 18.4 Project Refresh Cadence

Git/project cache SHOULD be refreshed based on:

- `HEAD` mtime
- `index` mtime
- configured TTL

## 19. Error Handling

### 19.1 Interactive Paths

Interactive render commands MUST fail gracefully.

They MUST NOT emit stack traces or large diagnostics into prompt/tmux output.

### 19.2 Missing Data

If data is unavailable:

- affected segment SHOULD be omitted
- or replaced with a minimal subtle placeholder

### 19.3 Diagnostic Surface

Debugging and failure analysis SHOULD occur through:

- `jetctx doctor`
- `jetctx inspect host`
- `jetctx inspect project`

## 20. Implementation Language

Rust is the preferred implementation language because it provides:

- fast startup
- good filesystem/process performance
- single-binary deployment
- strong typing for config/cache models
- good cross-platform support

Go is an acceptable alternative, but Rust is preferred for this design.

## 21. Suggested Rust Project Layout

```text
jetctx/
  Cargo.toml
  src/
    main.rs
    cli.rs
    config.rs
    cache.rs
    theme.rs
    render/
      mod.rs
      prompt.rs
      tmux.rs
    detect/
      mod.rs
      project.rs
      git.rs
      runtime.rs
      host.rs
    platform/
      mod.rs
      macos.rs
      linux.rs
```

### Module Roles

- `cli.rs`: argument parsing and command dispatch
- `config.rs`: config/env/CLI override resolution
- `cache.rs`: cache file IO, freshness, locking
- `theme.rs`: theme loading and resolved palette model, with search order that prefers `~/.config/jetctx/themes/` before project-local bundled themes
- `detect/project.rs`: project root and marker detection
- `detect/git.rs`: git state detection
- `detect/runtime.rs`: runtime/env detection
- `detect/host.rs`: host/system state collection
- `render/prompt.rs`: prompt renderer
- `render/tmux.rs`: tmux renderer
- `platform/*`: OS-specific host collectors

## 22. v1 Scope

### 22.1 Must-Have

v1 MUST implement:

- config loading
- theme loading
- prompt rendering
- tmux rendering
- project root detection
- git branch + minimal dirty summary
- host cache for battery/time/CPU/memory
- `nightowl` and `shaman` themes

### 22.2 Nice-to-Have

v1 MAY also include:

- network summary
- active-pane project summary in tmux
- runtime cache
- inspect/debug JSON output

### 22.3 Deferred

Later versions MAY add:

- background update daemons/timers
- richer git state
- more runtimes
- more themes
- more segment types

## 23. Example Outputs

### 23.1 Prompt Examples

Clean repo:

```text
dotfiles △ feature/theme rs 1.78
◎
```

Dirty repo + venv + failed command:

```text
api △ feat/auth ●2 ▪1 py .venv ◄ 842ms
○
```

SSH/root shell:

```text
root@host infra △ hotfix
◎
```

### 23.2 tmux Examples

Night Owl style:

```text
dev  1 editor                                   dotfiles △ main   BAT 82%  CPU 8%  MEM 16G  10:42
```

Shaman style:

```text
work  2 shell                                   infra △ prod      BAT 82%  CPU 8%  MEM 16G  10:42
```

## 24. Open Questions

The following questions remain open for implementation:

1. Should prompt rendering ever trigger asynchronous refresh, or only return cached/stale data?
2. Should tmux active-pane cwd be passed explicitly, or inferred internally?
3. Should weather be part of v1, or deferred?
4. Should network summary be:
   - simple online/offline
   - transport-aware
   - SSID-aware
5. Should git dirty summary use direct index inspection only, or one bounded git porcelain command?

## 25. Final Statement

`jetctx` v1 is a small, shared terminal context engine.

It exists to establish a clean ownership model:

- prompt for immediate project/command context
- tmux for ambient session/system context

and to implement that model with:

- one binary
- one cache
- one theme system
- low latency
- no redundant expensive work