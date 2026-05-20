## ADDED Requirements

### Requirement: Workflow run list display
The system SHALL display a table of GitHub Actions workflow runs for the selected repository with columns: Name, Status, Branch, Event, Time.

#### Scenario: Actions loaded
- **WHEN** actions panel is activated or refreshed
- **THEN** system fetches workflow runs for the selected repo and displays them in a navigable table

#### Scenario: Status indicators
- **WHEN** workflow runs are displayed
- **THEN** each run shows status with color/icon: green (success), red (failure), yellow (in_progress), gray (queued/cancelled)

### Requirement: Workflow job detail
The system SHALL show individual jobs and steps for a selected workflow run.

#### Scenario: Expand run
- **WHEN** user presses `Enter` on a selected workflow run
- **THEN** detail pane expands to show individual jobs with their status

### Requirement: Workflow log preview
The system SHALL show log output preview for selected jobs in the detail pane.

#### Scenario: View logs
- **WHEN** user navigates to a job and presses `v`
- **THEN** detail pane shows log output for that job, scrollable with j/k

### Requirement: Workflow actions
The system SHALL support re-running and cancelling workflow runs.

#### Scenario: Re-run failed
- **WHEN** user presses `r` on a failed workflow run
- **THEN** system triggers a re-run of the failed workflow via API

#### Scenario: Cancel run
- **WHEN** user presses `c` on an in-progress workflow run
- **THEN** system cancels the workflow run via API after confirmation
