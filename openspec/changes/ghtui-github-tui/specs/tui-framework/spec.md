## ADDED Requirements

### Requirement: Async event loop
The system SHALL run an async event loop using tokio that multiplexes terminal input events, periodic ticks, and API result messages.

#### Scenario: Event multiplexing
- **WHEN** app is running
- **THEN** system handles crossterm key events, 250ms tick interval, and API response channels concurrently via `tokio::select!`

#### Scenario: Non-blocking API calls
- **WHEN** user triggers an action that requires API data
- **THEN** system spawns async task, continues rendering, updates UI when result arrives

### Requirement: Terminal lifecycle
The system SHALL initialize the terminal in raw mode with alternate screen on startup, and restore normal mode on exit.

#### Scenario: Clean startup
- **WHEN** app starts
- **THEN** terminal switches to alternate screen, enables raw mode, hides cursor

#### Scenario: Clean shutdown
- **WHEN** user presses `q` or Ctrl+C
- **THEN** terminal restores normal mode, shows cursor, returns to main screen

#### Scenario: Panic recovery
- **WHEN** app panics
- **THEN** terminal is restored to normal mode before panic message is printed

### Requirement: Multi-panel layout
The system SHALL render a consistent layout with tab bar (top), content area (middle, split left/right), and status bar (bottom).

#### Scenario: Standard layout
- **WHEN** any panel is active
- **THEN** screen shows tab bar at top, list/table on left (~40%), detail pane on right (~60%), status bar at bottom

#### Scenario: Panel focus styling
- **WHEN** a panel is focused
- **THEN** its border is highlighted (active color) and other panels show inactive border color

### Requirement: TEA message routing
The system SHALL route messages to the appropriate panel's update function based on the active panel.

#### Scenario: Panel-specific input
- **WHEN** user presses a key while a specific panel is focused
- **THEN** the key event is routed to that panel's update function

#### Scenario: Global messages
- **WHEN** a message applies globally (quit, tab switch, refresh)
- **THEN** the top-level update handler processes it without delegating to a panel
