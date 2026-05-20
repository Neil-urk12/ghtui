## ADDED Requirements

### Requirement: Notification list display
The system SHALL display a table of GitHub notifications with columns: Repo, Subject, Type, Reason, Time.

#### Scenario: Notifications loaded
- **WHEN** notifications panel is activated or refreshed
- **THEN** system fetches notifications via API and displays them in a navigable table

#### Scenario: Unread indicators
- **WHEN** notifications are displayed
- **THEN** unread notifications are visually distinct (bold text or highlight)

### Requirement: Notification actions
The system SHALL support managing notifications via keyboard shortcuts.

#### Scenario: Mark as read
- **WHEN** user presses `x` on a selected notification
- **THEN** system marks that notification as read via API and updates the display

#### Scenario: Mark all as read
- **WHEN** user presses `X`
- **THEN** system marks all visible notifications as read

#### Scenario: Navigate to subject
- **WHEN** user presses `Enter` on a notification
- **THEN** system navigates to the relevant panel (PRs or Issues) for that notification's subject

### Requirement: Notification filtering
The system SHALL support filtering notifications by read state.

#### Scenario: Filter by read state
- **WHEN** user presses `1` (unread) or `2` (all)
- **THEN** notification list filters accordingly
