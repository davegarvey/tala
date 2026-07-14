## ADDED Requirements

### Requirement: Daemon starts and binds to a random available port
The system SHALL start a background HTTP server on a random available port on localhost.

#### Scenario: Start daemon writes port to daemon.json
- **WHEN** user runs `chit start`
- **THEN** the daemon SHALL start in the background
- **THEN** the daemon SHALL write its PID and port to `~/.chit/daemon.json`

#### Scenario: Start daemon prints session ID
- **WHEN** user runs `chit start`
- **THEN** the CLI SHALL print the new session ID (e.g., `sess_zk4m2`)

#### Scenario: Start daemon with initial message
- **WHEN** user runs `chit start "hello"`
- **THEN** the CLI SHALL create a session and send "hello" as the first message
- **THEN** the CLI SHALL block waiting for a reply

### Requirement: Daemon discovery via ~/.chit/daemon.json
The system SHALL use `~/.chit/daemon.json` for all CLI commands to locate the running daemon.

#### Scenario: Send finds daemon via daemon.json
- **WHEN** user runs `chit send "message"` and daemon.json exists with a live PID
- **THEN** the CLI SHALL read the port from daemon.json and send the request to that port

#### Scenario: Stale daemon.json triggers restart
- **WHEN** user runs a chit command and daemon.json points to a dead PID
- **THEN** the CLI SHALL detect the stale PID
- **THEN** the CLI SHALL start a new daemon and update daemon.json
- **THEN** the CLI SHALL proceed with the original command against the new daemon

#### Scenario: No daemon.json starts fresh
- **WHEN** user runs a chit command and daemon.json does not exist
- **THEN** the CLI SHALL start a new daemon
- **THEN** the CLI SHALL create daemon.json
- **THEN** the CLI SHALL proceed with the original command

### Requirement: Daemon idle timeout
The daemon SHALL terminate itself after a configurable period without any message activity.

#### Scenario: Daemon shuts down after idle timeout
- **WHEN** no messages have been sent or received on any session for the configured idle period
- **THEN** the daemon SHALL gracefully shut down
- **THEN** the daemon SHALL remove or mark daemon.json as stale

#### Scenario: Activity resets idle timer
- **WHEN** a message is sent or received on any session
- **THEN** the idle timer SHALL reset

### Requirement: Explicit daemon stop
The system SHALL support explicit daemon shutdown via CLI.

#### Scenario: Stop command terminates daemon
- **WHEN** user runs `chit stop`
- **THEN** the CLI SHALL send SIGTERM to the daemon process
- **THEN** the daemon SHALL shut down gracefully

### Requirement: Daemon status reporting
The system SHALL report daemon status via CLI.

#### Scenario: Status shows daemon info
- **WHEN** user runs `chit status`
- **THEN** the CLI SHALL display the daemon PID, port, uptime, and active session count

#### Scenario: Status with no daemon
- **WHEN** user runs `chit status` and no daemon is running
- **THEN** the CLI SHALL report "no daemon running"
