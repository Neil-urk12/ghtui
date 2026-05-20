## ADDED Requirements

### Requirement: Configuration file loading
The system SHALL load configuration from `~/.config/ghtui/config.toml` with sensible defaults if file is missing.

#### Scenario: Config file exists
- **WHEN** app starts and `~/.config/ghtui/config.toml` exists
- **THEN** system loads and applies the configuration

#### Scenario: Config file missing
- **WHEN** app starts and no config file exists
- **THEN** system uses built-in defaults and starts normally

### Requirement: Theme configuration
The system SHALL support customizing colors for active/inactive borders, highlights, text, and status indicators.

#### Scenario: Custom theme
- **WHEN** config file specifies theme colors
- **THEN** UI renders with the specified color palette

### Requirement: Keybinding configuration
The system SHALL support customizing keyboard shortcuts for all actions.

#### Scenario: Custom keybinding
- **WHEN** config file remaps a key action
- **THEN** the new key triggers the action instead of the default

### Requirement: Default filter configuration
The system SHALL support setting default filter states for each panel.

#### Scenario: Default PR filter
- **WHEN** config specifies `pulls_state = "open"`
- **THEN** PRs panel shows only open PRs by default instead of all
