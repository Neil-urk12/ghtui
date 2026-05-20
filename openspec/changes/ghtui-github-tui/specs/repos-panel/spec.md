## ADDED Requirements

### Requirement: Repository list display
The system SHALL display a table of the authenticated user's repositories with columns: Name, Language, Stars, Updated.

#### Scenario: Repos loaded
- **WHEN** repos panel is activated or refreshed
- **THEN** system fetches user's repos via API and displays them in a navigable table

#### Scenario: Empty state
- **WHEN** user has no repositories
- **THEN** panel shows "No repositories found" placeholder

### Requirement: Repository detail pane
The system SHALL show repository details in the right pane when a repo is selected.

#### Scenario: Repo selected
- **WHEN** user navigates to a repo in the list
- **THEN** detail pane shows: description, language, stars, forks, default branch, visibility, last updated

### Requirement: Repository navigation
The system SHALL support vim-style navigation within the repos panel.

#### Scenario: List navigation
- **WHEN** user presses `j`/`k` or arrow keys
- **THEN** selection moves down/up in the repo list

#### Scenario: Navigate to PRs
- **WHEN** user presses `Enter` on a selected repo
- **THEN** system switches to PRs panel filtered to that repository

### Requirement: Repository filtering
The system SHALL support filtering repos by name.

#### Scenario: Filter repos
- **WHEN** user presses `/` and types a search string
- **THEN** repo list filters to repos matching the search term (fuzzy match)

### Requirement: Repository sort
The system SHALL support sorting repos by different fields.

#### Scenario: Toggle sort
- **WHEN** user presses `s`
- **THEN** repos cycle sort order: name -> stars -> updated -> name
