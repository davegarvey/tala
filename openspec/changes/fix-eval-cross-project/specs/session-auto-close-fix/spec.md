## ADDED Requirements

### Requirement: Sessions survive daemon idle timeout
The system SHALL NOT lose session data when the daemon shuts down due to idle timeout. Open sessions SHALL be persisted to disk so they can be reloaded when the daemon restarts.

#### Scenario: Daemon restarts after idle timeout
- **WHEN** the daemon has open sessions
- **AND** the daemon shuts down due to idle timeout
- **THEN** the open sessions SHALL be persisted to disk
- **WHEN** the daemon starts again
- **THEN** the persisted sessions SHALL be available and listed as active

#### Scenario: Daemon does not lose sessions on SIGTERM
- **WHEN** the daemon receives SIGTERM
- **AND** has open sessions
- **THEN** the sessions SHALL be persisted before exit

### Requirement: Default idle timeout increased
The default idle timeout for the daemon SHALL be 86400 seconds (24 hours) instead of 600 seconds, to prevent unintended session loss during normal work sessions.

#### Scenario: Default idle timeout
- **WHEN** a user starts a session
- **AND** no activity occurs for 23 hours
- **THEN** the daemon SHALL still be running and the session SHALL still be open

### Requirement: Sessions not silently closed
Session state SHALL only change to `closed` when a user explicitly runs `tala close` or the equivalent API call. No background process SHALL mark sessions as closed without explicit user action.

#### Scenario: No silent close on daemon restart
- **WHEN** the daemon restarts and reloads persisted sessions
- **THEN** the reloaded sessions SHALL have `closed: false` and be fully usable
