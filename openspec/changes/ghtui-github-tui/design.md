## Context

Greenfield Rust project. Empty repo at `/home/imisenak/Documents/projects/ghtui`. Goal: build a lazygit-style TUI for GitHub covering repos, PRs, issues, notifications, and actions/workflows. No existing Rust tool covers this full scope.

Research findings:
- **ratatui 0.30**: dominant Rust TUI lib, immediate-mode rendering, modular workspace (ratatui-core, ratatui-widgets, ratatui). Uses crossterm backend by default.
- **TEA pattern**: ratatui has official docs for Elm Architecture (Model/Update/View). Model = App state, Update = message handler returning Cmd, View = render function.
- **gh-dash** (Go/Bubble Tea): closest existing tool, covers PRs/issues only. Good UX reference for layout and keybindings.
- **gitui** (Rust/tui-rs): Rust TUI for local git. Validates ratatui ecosystem viability.
- **lazygit** (Go/gocui): gold standard for multi-panel TUI UX. Context-aware keybindings, panel navigation.

## Goals / Non-Goals

**Goals:**
- Full GitHub experience in one TUI: repos, PRs, issues, notifications, actions
- Vim-style navigation with context-aware keybindings per panel
- Async API calls that don't block UI
- Auth via `gh` CLI or `GITHUB_TOKEN` env var
- TOML config for themes, keybindings, defaults
- Cross-platform (Linux, macOS, Windows)

**Non-Goals:**
- Git operations (staging, committing, branching) — use lazygit/gitui
- GitHub Enterprise Server support in v1
- Inline editing of PR/issue bodies
- Real-time webhooks/polling — periodic refresh only
- Image/video rendering in terminal

## Decisions

### D1: ratatui + Elm Architecture (not gocui, not tui-realm)

**Choice**: ratatui with manual TEA pattern implementation.

**Why**: ratatui is the standard Rust TUI library (19k+ stars, active maintenance). TEA gives structured state management without framework overhead. tui-realm adds abstraction layer we don't need. gocui is Go-only.

**Alternatives considered**:
- tui-realm: React/Elm framework on ratatui. Adds component model, message bus, focus management. Rejected: extra abstraction for marginal benefit at 5-7 panels.
- bubbletea-rs: Direct Go Bubble Tea port. Rejected: smaller ecosystem, less idiomatic Rust.
- cursive: Retained-mode, different paradigm. Rejected: less popular, different mental model.

### D2: tokio async runtime with channel-based API results

**Choice**: tokio event loop with `tokio::select!` multiplexing crossterm events, tick intervals, and API result channels.

**Why**: GitHub API calls are inherently async. Blocking the UI for network I/O is unacceptable. tokio is the de facto Rust async runtime. Channel-based results let the UI update immediately when data arrives.

**Alternatives considered**:
- Synchronous with background threads: simpler but harder to coordinate. Rejected: tokio select is cleaner.
- async-std: viable but smaller ecosystem than tokio. Rejected: tokio dominance.

### D3: Enum-dispatched panels (not trait objects)

**Choice**: `PanelState` enum with variants per panel. Update/view functions dispatch via match.

**Why**: No heap allocation, no dynamic dispatch, exhaustive matching at compile time. Adding a panel means adding an enum variant + match arm. At 5-7 panels, this is manageable.

**Alternatives considered**:
- `Box<dyn Panel>` trait objects: more extensible but requires downcasting for panel-specific state. Rejected: unnecessary complexity at this scale.
- Macro-generated dispatch: over-engineered for 5-7 variants.

### D4: GitHub REST API v3 (not GraphQL v4)

**Choice**: REST API via reqwest with JSON deserialization.

**Why**: REST is simpler to implement, well-documented, sufficient for the queries we need. GraphQL would reduce round trips for complex queries but adds schema complexity.

**Alternatives considered**:
- GraphQL v4: fewer round trips, typed schema. Rejected: more complex client code, need query builder.
- `gh api` subprocess: leverages gh CLI auth. Rejected: spawning processes per request is slow, harder to cache.

### D5: Auth via gh CLI token resolution

**Choice**: `GITHUB_TOKEN` env -> `gh auth token` command -> error.

**Why**: Most GitHub developers have `gh` CLI installed. This reuses existing auth without storing credentials. Env var override for CI/custom setups.

### D6: TTL cache with per-endpoint intervals

**Choice**: In-memory HashMap cache with configurable TTL per resource type.

**Why**: GitHub API rate limits (5000/hr authenticated). Caching prevents unnecessary requests. Different resources have different volatility (repos: 5min, actions: 10s).

## Risks / Trade-offs

- **[Rate limiting]** → Cache aggressively, show rate limit status in UI, auto-refresh after reset
- **[Large repos/PRs]** → Paginate API responses, limit detail preview size
- **[Terminal compatibility]** → crossterm handles cross-platform, but some terminals may not support all features. Fallback gracefully.
- **[Markdown rendering]** → Full markdown in terminal is impractical. Minimal renderer: headings, bold/italic, code blocks, lists. Rest as plain text.
- **[Auth token expiry]** → v1: detect 401, show error, prompt restart. Future: re-resolve token.
