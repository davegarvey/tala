## ADDED Requirements

### Requirement: Closed-session send shows actionable error
When `tala send` targets a closed session, the system SHALL display a message suggesting the user reopen the session or start a new one, instead of a terse "session is closed" error.

#### Scenario: Send to closed session with explicit ID
- **WHEN** user runs `tala send --session <closed-id> "hello"`
- **THEN** the system SHALL print an error including the session ID and suggest `tala session reopen <closed-id>` or `tala start`
- **AND** the system SHALL exit with a non-zero status code

#### Scenario: Send via stale active session that was closed
- **WHEN** the active session file points to a closed session
- **AND** user runs `tala send "hello"`
- **THEN** the system SHALL detect the session is closed before attempting to send
- **AND** SHALL print an actionable error message with suggestions

### Requirement: Default wait timeout reduced to 60s
The default timeout for `tala wait` SHALL be 60 seconds instead of 300 seconds. The timeout SHALL remain configurable via the `default_timeout` field in the global config.

#### Scenario: Wait with no timeout argument
- **WHEN** user runs `tala wait <session>` without `--timeout`
- **THEN** the system SHALL wait up to 60 seconds for new messages before timing out

#### Scenario: Wait with explicit timeout argument
- **WHEN** user runs `tala wait --timeout 120 <session>`
- **THEN** the system SHALL wait up to 120 seconds

### Requirement: Wait command shows initial feedback with timeout
When `tala wait` begins waiting, the system SHALL print an immediate message to stderr confirming it is waiting and showing the timeout value.

#### Scenario: Wait with feedback
- **WHEN** user runs `tala wait <session> --timeout 30`
- **THEN** the system SHALL print `Waiting for messages in session <id> (timeout: 30s)...` to stderr before initiating the long poll

### Requirement: unread_count excludes own sent messages
The `unread_count` field in `tala list --json` output SHALL exclude messages sent by the local agent, even when no `.tala/config.json` file exists. The fallback sender name (directory name) SHALL be used for filtering.

#### Scenario: Unread count without project config
- **WHEN** no `.tala/config.json` file exists
- **AND** the user has sent messages in a session
- **WHEN** user runs `tala list --json`
- **THEN** the `unread_count` for that session SHALL exclude messages whose sender matches the current directory name

#### Scenario: Unread count with project config
- **WHEN** `.tala/config.json` has a `"name"` field
- **WHEN** user runs `tala list --json`
- **THEN** the `unread_count` SHALL exclude messages whose sender matches the configured name

### Requirement: CLI help clarifies wait/stream/listen distinction
The `--help` output for `tala wait`, `tala stream`, and `tala listen` SHALL include brief usage guidance explaining when to use each command instead of the others.

#### Scenario: Help text shows cross-references
- **WHEN** user runs `tala wait --help`
- **THEN** the help output SHALL describe that `wait` is for blocking poll, `stream` for real-time SSE on a single session, and `listen` for observing all sessions
