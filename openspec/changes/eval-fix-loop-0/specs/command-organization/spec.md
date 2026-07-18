## ADDED Requirements

### Requirement: tala session --help SHALL clarify relationship to top-level commands
The `tala session --help` output SHALL explicitly note that top-level shortcuts exist for `list` and `close` commands.

#### Scenario: session help shows shortcut hints
- **WHEN** user runs `tala session --help`
- **THEN** the help text for `list` SHALL include: "(alias: `tala list`)"
- **AND** the help text for `close` SHALL include: "(alias: `tala close`)"

### Requirement: tala use help text SHALL mention tala session
The help text for `tala use` SHALL mention that `tala session show/rename/reopen` are available for advanced session management.

#### Scenario: tala use --help mentions session subcommand
- **WHEN** user runs `tala use --help`
- **THEN** the help text SHALL include: "See also: tala session (show, rename, reopen)"

### Requirement: tala close without arg SHALL use active session
When `tala close` is called without a session argument, it SHALL close the currently active session (matching the behavior of other commands).

#### Scenario: close without arg closes active session
- **GIVEN** active session is `sess_abc`
- **WHEN** user runs `tala close`
- **THEN** session `sess_abc` SHALL be closed
- **AND** a confirmation SHALL be printed: "Session sess_abc: closed"

#### Scenario: close without arg and no active session
- **GIVEN** no active session is set
- **WHEN** user runs `tala close`
- **THEN** the system SHALL show an error: "No active session set"
- **AND** list available sessions with their IDs
