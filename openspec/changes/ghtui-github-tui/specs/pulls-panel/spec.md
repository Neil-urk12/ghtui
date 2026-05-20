## ADDED Requirements

### Requirement: Pull request list display
The system SHALL display a table of pull requests for the selected repository with columns: #, Title, Author, Status, Reviews, Updated.

#### Scenario: PRs loaded
- **WHEN** pulls panel is activated or refreshed
- **THEN** system fetches PRs for the selected repo and displays them in a navigable table

#### Scenario: PR status indicators
- **WHEN** PRs are displayed
- **THEN** each PR shows status with color: green (open), purple (merged), red (closed)

### Requirement: Pull request detail pane
The system SHALL show PR details in the right pane including body text and diff preview.

#### Scenario: PR selected
- **WHEN** user navigates to a PR in the list
- **THEN** detail pane shows PR body (rendered markdown) and first N lines of diff

### Requirement: Pull request actions
The system SHALL support common PR actions via keyboard shortcuts.

#### Scenario: Open in browser
- **WHEN** user presses `o` on a selected PR
- **THEN** system opens the PR URL in the default browser via `gh browse`

#### Scenario: Checkout PR branch
- **WHEN** user presses `c` on a selected PR
- **THEN** system runs `gh pr checkout` for the selected PR

#### Scenario: Approve PR
- **WHEN** user presses `a` on a selected PR
- **THEN** system submits an approval review via API

#### Scenario: Merge PR
- **WHEN** user presses `m` on a selected PR
- **THEN** system shows confirmation popup, then merges PR if confirmed

### Requirement: Pull request filtering
The system SHALL support filtering PRs by state (open/closed/all).

#### Scenario: Filter by state
- **WHEN** user presses `1` (open), `2` (closed), or `3` (all)
- **THEN** PR list filters to show only PRs in that state
