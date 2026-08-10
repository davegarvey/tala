## Purpose

Session management in tala: sessions are created with unique auto-generated IDs, listed, closed, reopened, and renamed, with auto-targeting when a session is optional and names that persist across daemon restarts.

## Requirements

### Requirement: Create session

The system SHALL create a new session with an auto-generated ID in the format `sess_<random>` when the user runs `tala session create`, and SHALL guarantee the ID is unique among all sessions. `tala send` auto-creation is specified in message-sending.

#### Scenario: Session create returns auto-generated ID

- **WHEN** user runs `tala session create`
- **THEN** a new session SHALL be created with an ID in the format `sess_<random>`
- **THEN** the session ID SHALL be printed on stdout
- **THEN** the session SHALL be listed in the daemon's sessions

#### Scenario: Session IDs are unique

- **WHEN** multiple sessions are created
- **THEN** each session SHALL have a unique ID

#### Scenario: Create with a name

- **WHEN** user runs `tala session create --name "review"` and no other session is named "review"
- **THEN** the session SHALL be created with name "review"
- **WHEN** user runs `tala session create --name "review"` and another session is already named "review"
- **THEN** the command SHALL error without creating a session

### Requirement: List sessions

The system SHALL list all sessions, showing each session's ID, name, status (open/closed), and message count.

#### Scenario: List shows all sessions

- **WHEN** user runs `tala list` and sessions exist
- **THEN** the CLI SHALL print each session ID, name, status, and message count

#### Scenario: List with no sessions

- **WHEN** user runs `tala list` and no sessions exist
- **THEN** the CLI SHALL print "No sessions"

### Requirement: Close session

The system SHALL close an open session, marking it closed so it is no longer targetable for new messages. Closing SHALL broadcast the daemon's session-closed event; waiters on that session are notified per message-waiting.

#### Scenario: Close session marks it closed

- **WHEN** user runs `tala close <session>`
- **THEN** the session SHALL be marked as closed
- **THEN** the CLI SHALL print a confirmation ("Session <id>: closed")
- **THEN** subsequent `tala list` SHALL show the session as closed

#### Scenario: Close with no session ID auto-targets a single session

- **WHEN** user runs `tala close` and exactly one open session exists
- **THEN** that session SHALL be closed
- **WHEN** user runs `tala close` and multiple open sessions exist
- **THEN** the CLI SHALL error with a list of the available sessions
- **WHEN** user runs `tala close` and no open sessions exist
- **THEN** the CLI SHALL error directing the user to `tala send`

### Requirement: Auto-target single session

When a session ID is optional and exactly one open session exists, commands SHALL target that session automatically. Send and wait auto-targeting are specified in message-sending and message-waiting; this requirement covers the remaining session-scoped commands.

#### Scenario: Close and history auto-target a single session

- **WHEN** user runs `tala close` or `tala history` without a session ID and exactly one open session exists
- **THEN** the command SHALL apply to that session

#### Scenario: No sessions produces clear error

- **WHEN** user runs a session-scoped command without a session ID and no open sessions exist
- **THEN** the CLI SHALL print a clear error message directing the user to `tala send`

### Requirement: `tala session reopen` reopens a closed session

`tala session reopen <id>` SHALL set a closed session's `closed` field to `false` and update its `last_activity` timestamp. After reopening, the session SHALL accept new messages, with message IDs continuing from the session's existing sequence. The daemon SHALL broadcast the reopen event (see daemon).

#### Scenario: Reopen a closed session

- **WHEN** user runs `tala session reopen sess_abc` on a closed session
- **THEN** the session SHALL be reopened (closed = false)
- **THEN** stdout SHALL contain "Session sess_abc reopened"

#### Scenario: Send to reopened session

- **WHEN** a session is closed, then reopened via `tala session reopen`
- **AND** user runs `tala send --session sess_abc "new message"`
- **THEN** the message SHALL be accepted and stored in the session with a sequential ID

#### Scenario: Reopen an already-open session

- **WHEN** user runs `tala session reopen sess_abc` on a session that is already open
- **THEN** the command SHALL succeed silently (no error, no state change, no event broadcast)

#### Scenario: Reopen a non-existent session

- **WHEN** user runs `tala session reopen nonexistent`
- **THEN** the command SHALL error with a "session 'nonexistent' not found" message

### Requirement: `tala session reopen --json` output

`tala session reopen` SHALL accept a `--json` / `-j` flag. When set, the response SHALL be a JSON object with `session_id` and `status` fields.

#### Scenario: Reopen with --json

- **WHEN** user runs `tala session reopen sess_abc --json` on a closed session
- **THEN** stdout SHALL contain `{"session_id": "sess_abc", "status": "reopened"}`
- **THEN** the human-readable "Session sess_abc reopened" text SHALL NOT appear on stdout

### Requirement: Session name persists across daemon restarts

The daemon SHALL persist session names to disk so they survive restarts, and SHALL NOT overwrite a custom name when the session receives messages.

#### Scenario: Rename survives daemon restart

- **WHEN** user renames a session with `tala session rename <id> <newname>`
- **AND** daemon is restarted
- **AND** user runs `tala list`
- **THEN** the session name SHALL be `<newname>`

#### Scenario: Session name not overwritten by counterparty message

- **WHEN** user renames a session to `<customname>`
- **AND** counterparty sends a message in that session
- **THEN** the session name SHALL remain `<customname>`

#### Scenario: Sessions file written on rename and loaded on start

- **WHEN** user renames a session
- **THEN** `{TALA_HOME}/sessions.json` SHALL be written with the session map including the new name
- **WHEN** daemon starts and `{TALA_HOME}/sessions.json` exists
- **THEN** the daemon SHALL load the persisted session names from it

### Requirement: `tala use` shows session details

`tala use` SHALL display the active session's ID, name, and message count, both when showing the current active session and when setting a new one. The no-active-session listing case is specified in cli.

#### Scenario: Show active session with details

- **WHEN** user runs `tala use` with an active session set
- **THEN** the output SHALL include the session ID, session name, and total message count

#### Scenario: Set active session with confirmation

- **WHEN** user runs `tala use <session-id>`
- **THEN** the output SHALL include the session ID, session name, and message count

### Requirement: `tala close` clears a closed active session

When the session being closed is the active session, `tala close` SHALL clear the `.tala/active-session` file and warn the user, so a bare `tala send` never targets a closed session.

#### Scenario: Close active session prints warning

- **GIVEN** active session is `sess_abc`
- **WHEN** user runs `tala close sess_abc`
- **THEN** the session SHALL be closed
- **AND** the active session SHALL be cleared
- **AND** a warning SHALL be printed: "Active session was closed and cleared. Use `tala use <session-id>` to set a new one."

#### Scenario: Close non-active session

- **GIVEN** active session is `sess_abc`
- **WHEN** user runs `tala close sess_def`
- **THEN** the session SHALL be closed
- **AND** the active session SHALL remain `sess_abc`
- **AND** no active-session warning SHALL be printed
