## 1. Project Scaffold

- [x] 1.1 Initialize Cargo project with dependencies (ratatui, crossterm, tokio, reqwest, serde, color-eyre, clap, chrono, tracing, toml, dirs, futures)
- [x] 1.2 Create module structure: app.rs, cmd.rs, event.rs, run.rs, api/, panels/, widgets/, util/, config.rs, theme.rs, keys.rs, error.rs
- [x] 1.3 Implement main.rs: CLI arg parsing (clap), terminal init/restore, run loop entry

## 2. Core TEA Framework

- [x] 2.1 Define Event enum (Input, Tick, ApiResult), Message enum (navigation, panel messages, data loaded, quit), Cmd enum (None, fetch commands, custom future)
- [x] 2.2 Implement App struct with active_panel, panels HashMap, api client, config, should_quit
- [x] 2.3 Implement async event loop in run.rs: tokio::select! over crossterm EventStream, tick interval, API result channel
- [x] 2.4 Implement update() function routing messages to top-level handlers and panel dispatch
- [x] 2.5 Implement view() function: tab bar, content area split, status bar

## 3. Theme and Widgets

- [x] 3.1 Implement Theme struct with precomputed Style values (active/inactive borders, highlights, muted text, status colors)
- [x] 3.2 Implement tab_bar widget: renders panel tabs, highlights active tab
- [x] 3.3 Implement status_bar widget: shows mode, keybinding hints
- [x] 3.4 Implement loading spinner widget
- [x] 3.5 Implement empty_state placeholder widget
- [x] 3.6 Implement detail_pane widget (right-side preview with scroll)
- [x] 3.7 Implement popup widget (confirmation dialog, error toast, help overlay)

## 4. GitHub API Client

- [x] 4.1 Implement auth.rs: resolve token from GITHUB_TOKEN env, GH_TOKEN env, `gh auth token` command
- [x] 4.2 Implement client.rs: reqwest wrapper with auth headers, base URL, rate limit tracking from response headers
- [x] 4.3 Implement cache.rs: generic TTL cache (HashMap<Key, (Instant, T)>), per-endpoint TTL config
- [x] 4.4 Implement models.rs: serde types for Repo, PullRequest, Issue, Notification, WorkflowRun, WorkflowJob
- [x] 4.5 Implement repos.rs: GET /user/repos with pagination
- [x] 4.6 Implement pulls.rs: GET /repos/{owner}/{repo}/pulls with state filter
- [x] 4.7 Implement issues.rs: GET /repos/{owner}/{repo}/issues, POST comment
- [x] 4.8 Implement notifications.rs: GET /notifications, PATCH mark-as-read
- [x] 4.9 Implement actions.rs: GET /repos/{owner}/{repo}/actions/runs, POST re-run, POST cancel

## 5. Repos Panel

- [x] 5.1 Define ReposPanel struct (repos Vec, ListState, loading, error, detail_scroll, filter)
- [x] 5.2 Implement repos update: handle navigation (j/k/Enter), refresh (r), sort toggle (s), filter (/)
- [x] 5.3 Implement repos view: table with Name/Language/Stars/Updated columns, detail pane with repo info
- [x] 5.4 Wire repos panel into App: PanelKind::Repos, PanelState::Repos variant, message dispatch

## 6. Pulls Panel

- [x] 6.1 Define PullsPanel struct (pulls Vec, TableState, loading, error, state filter, repo context)
- [x] 6.2 Implement pulls update: navigation, state filter (1/2/3), open browser (o), checkout (c), approve (a), merge (m with confirm popup)
- [x] 6.3 Implement pulls view: table with #/Title/Author/Status/Reviews/Updated, detail pane with PR body + diff preview
- [x] 6.4 Wire pulls panel into App, handle ReposPanel Enter -> navigate to PRs for that repo

## 7. Issues Panel

- [x] 7.1 Define IssuesPanel struct (issues Vec, TableState, loading, error, state filter, repo context)
- [x] 7.2 Implement issues update: navigation, state filter (1/2/3), open browser (o), comment (c with input modal), assign (a)
- [x] 7.3 Implement issues view: table with #/Title/Author/Labels/Updated, detail pane with issue body
- [x] 7.4 Implement input widget for comment composition
- [x] 7.5 Wire issues panel into App

## 8. Notifications Panel

- [x] 8.1 Define NotificationsPanel struct (notifications Vec, ListState, loading, error, filter)
- [x] 8.2 Implement notifications update: navigation, mark read (x), mark all read (X), navigate to subject (Enter), filter (1/2)
- [x] 8.3 Implement notifications view: table with Repo/Subject/Type/Reason/Time, unread highlighting
- [x] 8.4 Wire notifications panel into App, handle Enter -> navigate to relevant PR/Issue panel

## 9. Actions Panel

- [x] 9.1 Define ActionsPanel struct (runs Vec, TableState, loading, error, expanded run, job selection)
- [x] 9.2 Implement actions update: navigation, expand run (Enter), view logs (v), re-run (r), cancel (c with confirm)
- [x] 9.3 Implement actions view: table with Name/Status/Branch/Event/Time, status color icons, expanded job list in detail pane
- [x] 9.4 Wire actions panel into App

## 10. Configuration

- [x] 10.1 Implement config.rs: TOML deserialization for ThemeConfig, KeybindingsConfig, DefaultsConfig, ApiConfig
- [x] 10.2 Create default.toml with sensible defaults
- [x] 10.3 Load config from ~/.config/ghtui/config.toml at startup, fall back to defaults
- [x] 10.4 Implement keys.rs: keybinding resolution from config, map key strings to crossterm KeyEvents
- [x] 10.5 Update theme.rs to build from ThemeConfig

## 11. Search and Filtering

- [x] 11.1 Implement fuzzy.rs: fuzzy string matching for search
- [x] 11.2 Add search state to App (active search, search text, which panel)
- [x] 11.3 Wire / key to activate search bar, Esc to clear
- [x] 11.4 Apply fuzzy filter to repos, pulls, issues panels

## 12. Utilities

- [x] 12.1 Implement format.rs: relative date display, number abbreviation (1.2k stars)
- [x] 12.2 Implement markdown.rs: minimal markdown -> ratatui Text (headings, bold, italic, code blocks, lists)

## 13. Polish and Error Handling

- [x] 13.1 Implement error.rs: error types, user-facing error messages
- [x] 13.2 Add error toast display for API failures
- [x] 13.3 Add rate limit display in status bar
- [x] 13.4 Handle edge cases: empty lists, no network, terminal resize
- [x] 13.5 Add help overlay (?) showing all keybindings
