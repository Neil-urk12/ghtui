## ADDED Requirements

### Requirement: Issue list display
The system SHALL display a table of issues for the selected repository with columns: #, Title, Author, Labels, Updated.

#### Scenario: Issues loaded
- **WHEN** issues panel is activated or refreshed
- **THEN** system fetches issues for the selected repo and displays them in a navigable table

#### Scenario: Issue state indicators
- **WHEN** issues are displayed
- **THEN** each issue shows state with color: green (open), red (closed)

### Requirement: Issue detail pane
The system SHALL show issue details in the right pane including body text.

#### Scenario: Issue selected
- **WHEN** user navigates to an issue in the list
- **THEN** detail pane shows issue body (rendered markdown), labels, assignees

### Requirement: Issue actions
The system SHALL support common issue actions via keyboard shortcuts.

#### Scenario: Open in browser
- **WHEN** user presses `o` on a selected issue
- **THEN** system opens the issue URL in the default browser

#### Scenario: Add comment
- **WHEN** user presses `c` on a selected issue
- **THEN** system shows text input modal for composing a comment, submits via API on Enter

#### Scenario: Assign self
- **WHEN** user presses `a` on a selected issue
- **THEN** system toggles self-assignment on the issue

### Requirement: Issue filtering
The system SHALL support filtering issues by state (open/closed/all).

#### Scenario: Filter by state
- **WHEN** user presses `1` (open), `2` (closed), or `3` (all)
- **THEN** issue list filters to show only issues in that state
