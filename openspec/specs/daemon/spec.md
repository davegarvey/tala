## Purpose

Daemon lifecycle in tala: a background HTTP server runs on localhost, is located by CLI commands via `daemon.json`, shuts down after an idle period or on explicit stop, persists sessions across restarts, and streams session lifecycle events to SSE consumers.

## Requirements

### Requirement: Daemon starts and binds to a random available port

The system SHALL run a background HTTP server on a random available port on localhost. The CLI SHALL start the daemon automatically when a command needs it and no live daemon is running.

#### Scenario: Start daemon writes daemon.json

- **WHEN** the daemon starts
- **THEN** it SHALL bind to a random available port on 127.0.0.1
- **THEN** it SHALL write its PID, host, port, start time, and protocol version to `{TALA_HOME}/daemon.json`

#### Scenario: CLI spawns daemon on demand

- **WHEN** user runs a tala command and no daemon is running
- **THEN** the CLI SHALL spawn the daemon as a background process
- **THEN** the CLI SHALL wait for `daemon.json` to appear and proceed with the original command

### Requirement: Daemon discovery via daemon.json

The system SHALL use `{TALA_HOME}/daemon.json` for all CLI commands to locate the running daemon, restarting it when the recorded daemon is stale or absent.

#### Scenario: Send finds daemon via daemon.json

- **WHEN** user runs `tala send "message"` and daemon.json exists with a live daemon
- **THEN** the CLI SHALL read the port from daemon.json and send the request to that port

#### Scenario: Stale daemon.json triggers restart

- **WHEN** user runs a tala command and daemon.json points to a dead daemon
- **THEN** the CLI SHALL detect the stale daemon.json and remove it
- **THEN** the CLI SHALL start a new daemon and wait for the updated daemon.json
- **THEN** the CLI SHALL proceed with the original command against the new daemon

#### Scenario: No daemon.json starts fresh

- **WHEN** user runs a tala command and daemon.json does not exist
- **THEN** the CLI SHALL start a new daemon, which SHALL create daemon.json
- **THEN** the CLI SHALL proceed with the original command

### Requirement: Daemon idle timeout

The daemon SHALL terminate itself after a configurable period without any session activity. The default idle timeout SHALL be 86400 seconds (24 hours), configurable via the user config's `idle_timeout` value.

#### Scenario: Daemon shuts down after idle timeout

- **WHEN** no session has had activity for the configured idle period
- **THEN** the daemon SHALL persist its state and shut down

#### Scenario: Activity resets idle timer

- **WHEN** a message is sent or received in any session
- **THEN** the idle timer SHALL reset

#### Scenario: Default idle timeout keeps sessions alive

- **WHEN** a user starts a session
- **AND** no activity occurs for 23 hours
- **THEN** the daemon SHALL still be running and the session SHALL still be open

### Requirement: Explicit daemon stop

The system SHALL support explicit daemon shutdown via `tala stop`.

#### Scenario: Stop command terminates daemon

- **WHEN** user runs `tala stop`
- **THEN** the CLI SHALL send SIGTERM to the daemon process
- **THEN** the daemon SHALL shut down gracefully, persisting state and removing daemon.json
- **THEN** the CLI SHALL print "daemon stopped"

#### Scenario: Stop with no daemon

- **WHEN** user runs `tala stop` and no daemon.json exists
- **THEN** the CLI SHALL print "daemon is not running"

### Requirement: Daemon status reporting

The system SHALL report daemon status via `tala status`. Protocol version reporting is specified in daemon-compat.

#### Scenario: Status shows daemon info

- **WHEN** user runs `tala status` and a daemon is running
- **THEN** the CLI SHALL display the daemon PID, port, host, start time, and unread message count

#### Scenario: Status with no daemon

- **WHEN** user runs `tala status` and no daemon is running
- **THEN** the CLI SHALL report "no daemon running (checked {TALA_HOME}/daemon.json)"

### Requirement: Sessions survive daemon shutdown

The system SHALL NOT lose session data when the daemon shuts down, whether from idle timeout or SIGTERM. Open sessions SHALL be persisted to disk and reloaded when the daemon restarts.

#### Scenario: Daemon restarts after idle timeout

- **WHEN** the daemon has open sessions
- **AND** the daemon shuts down due to idle timeout
- **THEN** the open sessions SHALL be persisted to disk
- **WHEN** the daemon starts again
- **THEN** the persisted sessions SHALL be available and listed as open

#### Scenario: Daemon persists sessions on SIGTERM

- **WHEN** the daemon receives SIGTERM
- **AND** has open sessions
- **THEN** the sessions SHALL be persisted before exit

#### Scenario: Sessions not silently closed

- **WHEN** the daemon restarts and reloads persisted sessions
- **THEN** the reloaded sessions SHALL have `closed: false` and be fully usable
- **THEN** session state SHALL only change to closed when a user explicitly runs `tala close` or the equivalent API call

### Requirement: Session rename broadcasts a renamed event

When a session is renamed, the daemon SHALL broadcast a `SessionRenamed` event carrying the session ID, old name, and new name, and SHALL emit a "renamed" observe event to SSE consumers on `tala listen`.

#### Scenario: Rename broadcasts to listen consumers

- **WHEN** a user renames a session via `tala session rename <session-id> <new-name>`
- **THEN** the daemon SHALL emit a `DaemonEvent::SessionRenamed` containing the session ID, old name, and new name
- **AND** clients connected via SSE (`tala listen`) SHALL receive a "renamed" event with the session ID and new session name

#### Scenario: `tala list` reflects rename after event

- **WHEN** a session is renamed
- **THEN** subsequent calls to `tala list` SHALL show the new session name

### Requirement: Session reopen broadcasts a reopened event

When a session is reopened, the daemon SHALL broadcast a `SessionReopened` event and SHALL emit a "reopened" observe event to SSE consumers on `tala listen`.

#### Scenario: Reopen broadcasts to listen consumers

- **WHEN** a user reopens a session via `tala session reopen <session-id>`
- **THEN** the daemon SHALL emit a `DaemonEvent::SessionReopened` containing the session ID
- **AND** clients connected via SSE (`tala listen`) SHALL receive a "reopened" event for that session
