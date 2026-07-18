## ADDED Requirements

### Requirement: Daemon discovery reads correct path
The system SHALL locate the daemon by reading `daemon.json` from `TALA_HOME` (defaulting to `~/.tala`).

#### Scenario: Discover uses TALA_HOME/daemon.json
- **WHEN** a user runs `tala discover`
- **THEN** the system SHALL read `daemon.json` from `{TALA_HOME}/daemon.json` (not `{project}/.tala/daemon.json`)
- **AND** the system SHALL report the daemon as running if the file exists and contains a valid PID

## MODIFIED Requirements

### Requirement: Daemon not found error message
The system SHALL produce descriptive error messages when the daemon cannot be located.

#### Scenario: TALA_HOME path does not exist
- **WHEN** `tala` cannot find the daemon
- **AND** the `TALA_HOME` path does not exist
- **THEN** the system SHALL display: "Daemon not found at {path}. Check that TALA_HOME is set correctly."
- **AND** the system SHALL NOT display "daemon failed to start within 5 seconds"

#### Scenario: daemon.json exists but daemon is not running
- **WHEN** `tala` cannot find the daemon
- **AND** the `daemon.json` file exists but the daemon process is not running
- **THEN** the system SHALL display: "Daemon at {path} is not running. Try starting it with 'tala daemon'."
