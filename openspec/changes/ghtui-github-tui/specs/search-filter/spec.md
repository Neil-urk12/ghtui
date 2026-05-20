## ADDED Requirements

### Requirement: Global search bar
The system SHALL provide a search bar activated by `/` that filters the active panel's content.

#### Scenario: Activate search
- **WHEN** user presses `/` in any panel
- **THEN** a text input appears at the bottom of the content area, ready for typing

#### Scenario: Filter results
- **WHEN** user types in the search bar
- **THEN** the active panel's list filters in real-time to show matching items (fuzzy match)

#### Scenario: Clear search
- **WHEN** user presses `Esc` or clears the search text
- **THEN** the filter is removed and all items are shown again

### Requirement: Per-panel filtering
Each panel SHALL support its own filter state that persists while navigating.

#### Scenario: Independent filters
- **WHEN** user sets a filter in repos panel, switches to PRs, then back to repos
- **THEN** repos filter is preserved and PRs panel has its own independent filter

### Requirement: Fuzzy matching
The system SHALL use fuzzy string matching for search/filter operations.

#### Scenario: Fuzzy match
- **WHEN** user searches for "auth" in repos panel
- **THEN** results include repos with "auth" anywhere in the name (e.g., "my-auth-service", "oauth2-lib")
