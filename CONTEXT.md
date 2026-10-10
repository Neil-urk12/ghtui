# CONTEXT.md — ghtui Domain Glossary

## Terms

**Panel** — A full-screen tab in the TUI (Repos, Pulls, Issues, Notifications, Actions). Each owns data, handles keys, and renders content.

**PanelItem** — Trait defining panel-specific behavior (data rendering, key handling, fetch commands). Each panel implements this; `ListPanel<T>` provides the generic `Panel` implementation.

**ListPanel** — Generic wrapper `ListPanel<T: PanelItem>` that implements `Panel` once. Owns selection state, detail pane, and delegates to `PanelItem` methods. Eliminates per-panel boilerplate.

**SelectionState** — Tracks which item is selected in a panel's list. Wraps either ListState or TableState. Also stores list length for navigation bounds.

**DetailPane** — Scroll state for the right-side detail view. A u16 offset.

**RenderContext** — Read-only params for `Panel::render`: `theme`, `detail_pane`, `tick`. Keeps mutable params (`frame`, `area`, `selection`) as direct arguments.

**ListView** — The reusable split-pane widget (list on left, detail on right). Renders loading, empty, and normal states.

**Cmd** — A requested side effect belonging to an originating Panel and, where applicable, a repository. Its outcome belongs to that origin regardless of which tab is currently shown.

**Message** — An instruction to transition the app's state, including input decisions and Cmd outcomes.

**Event** — Raw terminal input or a periodic tick.

**ApiPayload** — Data or an outcome returned from GitHub for a Cmd. Its owning Panel is independent of the tab currently shown.
**Status Bar** — Bottom row of the terminal. Shows per-panel key hints (left) and global hints (right).

**Detail Footer** — Muted key-hint line appended below the detail pane content.

**Empty State** — Centered message shown when a panel has no items loaded.

**Key Hint** — A terse `key:label` pair in the status bar or footer explaining a keyboard shortcut.
