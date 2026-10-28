# Changelog

## [Unreleased]

All development history through `a0bc784`, including the pending fixes, is collected below.

### Added

- Initial Rust terminal application with Ratatui, Crossterm, a Tokio event loop, tab navigation, list and detail panes, help, status messages, relative timestamps, number formatting, Markdown rendering, and fuzzy matching. (`5247976`)
- GitHub REST client, response models, authentication resolution, rate-limit tracking, and resource-specific caches. (`c2e60ac`)
- Repository, pull request, issue, notification, and Actions panels, with repository sorting, detail views, TOML configuration, theme colors, keyboard shortcuts, and default filters. (`7393f44`)
- Asynchronous command execution and GitHub mutations for approving and merging pull requests, reading notifications, and rerunning or cancelling workflows. (`b5b77f0`)
- In-app GitHub OAuth device flow, authorization codes and countdown, persistent token storage, forced login, and logout. (`33f4bf3`)
- Empty-state guidance, loading indicators, and automatically dismissed toast messages. (`8844434`)
- Search filtering across all five panels. (`d9e8047`)
- Confirmation dialogs for destructive actions. (`9b72f77`)
- Pull request reviews, conversation comments, inline code comments, and close/reopen actions. (`f2d4db0`)
- Separate errors for pull request detail requests and debounced comment fetching during navigation. (`8c52d31`)
- Pull request diff viewer with file patches, line numbers, themed additions and deletions, renamed-file headers, and placeholders for unavailable patches. (`dbdfb5b`)
- Panel-specific diff empty states. (`6c9e95a`)
- Fullscreen and collapsed detail-pane modes. (`9bc7eb5`)
- Shared pagination support for GitHub list endpoints. (`205931d`)
- Distinct merged pull request status, configurable merged-status colors, and guards against reopening merged requests or merging and closing requests that are already closed. (`09563fa`)

### Changed

- Extracted a shared panel interface and consolidated panel navigation and update handling. (`1dc1d27`, `09efcc2`)
- Separated application domain types from GitHub response models. (`208f7d3`)
- Split terminal event processing, rendering, HTTP helpers, and application state transitions into dedicated modules and functions. (`68e4fbe`, `94441e1`, `f781aec`)
- Replaced direct panel downcasts with interface dispatch; subsequently consolidated enum dispatch and owner/repository helpers, reduced delegation with a macro, and moved to boxed `Panel` trait implementations with panel-owned behavior. (`95db4ba`, `80b5367`, `378d91f`, `a0bc784`)
- Consolidated all five panels around a shared list-view widget, selection and detail-pane state, generic `ListPanelData`, and the `ListPanel<T: PanelItem>` wrapper; removed the intermediate `PanelEntry` wrapper and selection `RefCell`. (`3dd9dbf`, `17882ec`, `62e5569`, `0a3f06e`, `f39f518`)
- Centralized fetch commands and loading-state updates. (`a8eedb4`)
- Replaced string-based state filters with a typed enum and improved handling of invalid values and runtime failures. (`bd908e5`)
- Extracted a shared utility for spawning background processes. (`091e0fe`)
- Made destructive-action labels optional, added classification invariants, and unified confirmation requirements and labels in command behavior. (`2299c36`, `3f70484`)
- Standardized status-bar and detail-footer hints as descriptive `key:action` pairs. (`9baf064`)
- Moved cache invalidation into the GitHub client mutation methods. (`4fc4a15`)
- Shared repeated filter-key handling through helpers and the `StateFilterable` trait. (`752e74f`, `de2a9cc`)
- Extracted shared constants and clarified tests that depend on them. (`6b036cd`)
- Introduced `RepoContext` for commands that operate on a repository. (`3b58b0e`)
- Consolidated paginated fetch helpers, strengthened URL parsing, and extracted diff parsing utilities. (`7ba6a76`)
- Propagated HTTP-client and application-construction errors to callers. (`dbb1803`)
- Simplified command dispatch and list rendering, and removed unused response fields and dependencies. (`da55baa`)
- Adopted structured tracing fields for application errors. (`7c6d625`)

### Fixed

- Passed state filters through commands to API requests and corrected refresh/cache behavior and success feedback after mutations. (`4d2f3e2`)
- Passed an explicit repository to `gh browse` when opening details outside a repository checkout. (`016ebf5`)
- Prevented debug-build stack overflows by constructing the Tokio runtime with a larger thread stack. (`b24efe3`)
- Simplified redundant match guards and prefix handling, and removed an unused index. (`67a4a3d`)
- Prevented recursion in the default panel key handler. (`034ffd2`)
- Preserved valid default filter values instead of deriving empty strings for configuration defaults. (`0d4b9f6`)
- Suppressed GitHub CLI subprocess output that could disrupt the terminal interface and opened workflow runs in the browser. (`09b0d4c`)
- Removed the unimplemented Enter binding from notification help. (`6659a15`)
- Allowed retries after pull request comment-fetch failures while retaining navigation debounce. (`0cc514b`)
- Kept pull request detail fetching from replacing the panel with a loading state during navigation. (`0ffcce2`)
- Widened pull request and issue numbers to `u64` and refined related cache invalidation. (`976077e`)
- Removed redundant integer conversions on fields already represented as `u32`. (`9c95cf4`)
- Skipped redundant requests when the selected state filter is already active. (`089ac58`)
- Reported the actual byte count when truncating HTTP error messages. (`c035f02`)
- Dispatched the repository-fetch command returned after a successful login. (`7c6d625`)
- Kept login input responsive while polling and bounded HTTP request duration, including stalled response bodies. (`a8241e3`)
- Routed command results to their originating panel and repository, discarded superseded or stale reads, tracked loading against pending requests, and refreshed the originating panel after mutations. (`c9bb3bc`)
- Used UTF-8 byte offsets consistently in inline Markdown parsing so emoji and other multibyte characters around bold, italic, and code spans cannot cause invalid string slicing.
- Used `PUT /notifications` when marking all notifications as read and corrected the HTTP mock to expect PUT.
- Cached all and unread notifications separately and invalidated both variants after successful notification mutations.

### Security

- Validated pagination links and blocked cross-host redirects and pagination requests; applied redirect protection to the login flow as well. (`eda32c3`, `1adc8c3`, `ee5e3d3`)
- Gated Unix-specific token-file handling by platform and added authentication-module coverage. (`f6279a7`)
- Preserved authentication HTTP error details with bounded body reads and redacted access tokens and device codes from debug output. (`cca071b`)
- Enforced mode `0600` on opened token files, including existing files with broader permissions, before truncating or writing credentials; permission failures preserve the previous file contents.
- Isolated default token storage in a temporary directory per test thread so login tests cannot overwrite real saved credentials or race over configuration environment variables. Explicit storage tests continue to use temporary-directory helpers.

### Removed

- Removed dead code, unused API and workflow models, obsolete panel fields, and unsupported configuration options for API URLs, cache lifetimes, refresh intervals, navigation keys, and unused theme colors. (`f3991f7`, `da55baa`)

### Tests and tooling

- Added unit tests for pure functions across six modules. (`697d4c9`)
- Added tests for all five panel-item implementations. (`b40255f`)
- Extracted pure login functions and covered their state decisions with unit tests. (`976eeb0`)
- Added tracing-based assertions, a shared cache-seeding helper, warning-level mutex-poison diagnostics, and precise log assertions. (`052273d`, `3062bb3`)
- Added GitHub Actions checks for tests, Clippy, and formatting with pinned actions, restricted permissions, job timeouts, caching, and concurrency control. (`8a3e77a`)
- Added weekly Dependabot updates for Cargo and GitHub Actions, dependency auditing with `cargo audit`, and CI configuration self-tests. (`4c70e95`)
- Ignored generated build artifacts. (`1c67201`)
- Temporarily disabled automatic CI runs on pushes and pull requests while retaining manual `workflow_dispatch` runs. (`2314ec8`)
- Added regression coverage for temporary test configuration, existing token-file permissions, Unicode Markdown, notification filter caches, and invalidation of both notification variants.

### Documentation

- Added OpenSpec project configuration, proposal, design, implementation tasks, feature specifications, and development workflow commands and skills. (`5247976`)
- Added and updated the domain glossary for panel architecture and command-result ownership. (`c9bb3bc`, `a0bc784`)
