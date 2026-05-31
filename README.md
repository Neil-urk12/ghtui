# ghtui

A lazygit-style TUI for GitHub.

## Features

- Browse repos, pull requests, issues, notifications, and workflow runs
- View PR diffs with syntax-highlighted patches
- Filter by open/closed/all state
- Configurable keybindings and theme
- API caching with configurable TTLs

## Install

```bash
cargo install --path .
```

## Usage

```bash
ghtui            # Start TUI
ghtui --debug    # Enable debug logging
ghtui --login    # Re-authenticate
ghtui --logout   # Clear stored token
```

## Authentication

ghtui resolves tokens in order:
1. `GITHUB_TOKEN` environment variable
2. `GH_TOKEN` environment variable
3. `gh auth token` (GitHub CLI)
4. Stored token file (`~/.config/ghtui/auth.json`)
5. Device flow login prompt

## Configuration

Config file: `~/.config/ghtui/config.toml`

```toml
[defaults]
pulls_state = "open"             # "open", "closed", or "all"
issues_state = "open"            # "open", "closed", or "all"
notifications_show_all = false   # show all notifications, not just participating

[keybindings]
quit = "q"
refresh = "r"
next_tab = "Tab"
prev_tab = "Shift-Tab"
search = "/"
help = "?"

[theme]
active_border_color = "cyan"
inactive_border_color = "gray"
highlight_color = "blue"
muted_color = "gray"
success_color = "green"
error_color = "red"
diff_addition_color = "green"
diff_deletion_color = "red"
diff_header_color = "cyan"
merged_color = "magenta"
```

## Keybindings

| Key | Action |
|-----|--------|
| `j`/`k` | Navigate up/down |
| `Tab` / `Shift-Tab` | Switch panel forward/back |
| `Enter` | Select / expand |
| `o` | Open in browser |
| `s` | Sort |
| `1`/`2`/`3` | Filter open/closed/all |
| `d` | PR overview |
| `a` | Approve PR |
| `m` | Merge PR |
| `x`/`X` | Close/reopen PR |
| `v` | View workflow logs |
| `r` | Refresh |
| `M` | Mark all notifications read |
| `J`/`K` | Scroll detail pane |
| `f` | Fullscreen detail |
| `/` | Search |
| `?` | Help |
| `q` | Quit |

## License

MIT
