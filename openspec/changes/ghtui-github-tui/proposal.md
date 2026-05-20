## Why

No existing Rust TUI covers the full GitHub experience. gh-dash (Go) handles PRs/issues only. gitui (Rust) handles local git only. Users who want repos + PRs + issues + notifications + actions in one terminal tool have no Rust option. Building ghtui fills this gap with ratatui + TEA architecture.

## What Changes

- New Rust binary crate (`ghtui`) with ratatui TUI
- GitHub API client layer (REST v3) with auth via `gh` CLI / env var
- Multi-panel layout: Repos, Pulls, Issues, Notifications, Actions
- Elm Architecture (Model/Update/View) with async tokio event loop
- TOML config for themes, keybindings, default filters
- Vim-style navigation with context-aware keybindings per panel

## Capabilities

### New Capabilities

- `github-api-client`: Async GitHub REST API client with auth resolution, caching, rate limit tracking
- `tui-framework`: Core TEA loop (App/Message/Cmd), event multiplexing, terminal lifecycle
- `repos-panel`: Repository list with detail pane, navigation, filtering
- `pulls-panel`: Pull request list with status indicators, diff preview, actions (approve/merge/checkout)
- `issues-panel`: Issue list with body preview, labels, comment capability
- `notifications-panel`: Notification list with read/unread state, mark-as-read, type filtering
- `actions-panel`: Workflow run list with status icons, job detail, log preview
- `config-system`: TOML configuration for themes, keybindings, defaults, API settings
- `search-filter`: Global search bar, per-panel fuzzy filtering

### Modified Capabilities

(none - greenfield project)

## Impact

- New Rust crate with dependencies: ratatui, crossterm, tokio, reqwest, serde, color-eyre, clap, toml
- Requires `gh` CLI installed or `GITHUB_TOKEN` env var for authentication
- Terminal-only: no web UI, no server component
- Cross-platform via crossterm (Linux, macOS, Windows)
