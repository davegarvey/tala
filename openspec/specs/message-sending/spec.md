## Purpose

Message sending UX in tala: auto-created sessions announce themselves on stdout, JSON output stays clean, and starting a session with a message stores it exactly once.

## Requirements

### Requirement: Auto-create prints session ID to stdout

When `tala send` runs without an active session and auto-creates a new session, it SHALL print the new session ID to stdout (not stderr). JSON output SHALL NOT be interleaved with this text (the session_id is in the response body).

#### Scenario: Non-JSON auto-create visible
- **WHEN** user runs `tala send "hello"` with no active session and no `--json`
- **THEN** stdout SHALL contain the new session ID
- **THEN** the confirmation "✓ Sent message ..." SHALL also appear on stdout

#### Scenario: JSON auto-create not interleaved
- **WHEN** user runs `tala send "hello"` with `--json` and no active session
- **THEN** stdout SHALL contain only the JSON response
- **THEN** the session ID SHALL NOT be printed as plain text on stdout

### Requirement: Send with auto-create stores the message exactly once

When `tala send` auto-creates a session (no active session, zero open sessions), it SHALL create the session via `POST /api/sessions` with `message: None` and SHALL then send the message with a single `POST` to the new session. The message SHALL NOT be stored twice.

#### Scenario: Auto-create send has exactly one message
- **WHEN** user runs `tala send "hello"` with no active session and zero sessions on the daemon
- **THEN** `tala history` SHALL show exactly one message with content "hello"
- **WHEN** a second agent runs `tala wait --new` before the send
- **THEN** they SHALL receive exactly one message notification

### Requirement: `tala send` auto-creates a session when none exists

When `tala send` runs with no active session and no sessions exist on the daemon, it SHALL auto-create a new session via `POST /api/sessions`, write the new session ID to `.tala/active-session`, and send the message to the new session. The command SHALL print the new session ID to stdout. Auto-create SHALL NOT fire when an explicit `--session` is given, even if that session does not exist on the daemon.

#### Scenario: Send with no sessions auto-creates

- **WHEN** user runs `tala send "hello"` with no active session file and zero sessions on the daemon
- **THEN** a new session SHALL be created via `POST /api/sessions`
- **THEN** the message "hello" SHALL be sent to the new session
- **THEN** the new session ID SHALL be printed on stdout
- **THEN** `.tala/active-session` SHALL contain the new session ID

#### Scenario: Auto-create with --json output

- **WHEN** user runs `tala send "hello" --json` with no active session file and zero sessions on the daemon
- **THEN** the JSON response SHALL contain the `session_id` and message `id` fields
- **THEN** the session ID SHALL NOT be printed as plain text on stdout (it is included in JSON)

#### Scenario: Auto-create with --wait

- **WHEN** user runs `tala send "hello" --wait` with no active session and zero sessions on the daemon
- **THEN** a new session SHALL be auto-created
- **THEN** the message SHALL be sent and the command SHALL wait for a reply

#### Scenario: Auto-create does not fire with explicit --session flag

- **WHEN** user runs `tala send --session sess_abc "hello"` and `sess_abc` does not exist on the daemon
- **THEN** the command SHALL error (auto-create SHALL NOT fire for explicit session IDs)

### Requirement: `tala send` auto-replaces a stale active session

When `tala send` runs with a stale active session file (the session no longer exists on the daemon), it SHALL clear the stale reference, auto-create a new session, and write the new session ID to `.tala/active-session`.

#### Scenario: Send with stale active session auto-replaces

- **WHEN** user runs `tala send "hello"` with a stale active session file (session no longer exists on daemon)
- **THEN** the stale active session SHALL be cleared
- **THEN** a new session SHALL be auto-created (same as the zero-sessions case)
- **THEN** `.tala/active-session` SHALL contain the new session ID
- **THEN** the message "hello" SHALL be sent to the new session

### Requirement: `tala send` with no active session targets a single open session

When `tala send` runs with no active session and exactly one open session exists on the daemon, it SHALL send to that session. When multiple open sessions exist, it SHALL error and list the available sessions with their IDs and names.

#### Scenario: Send with a single open session auto-targets it

- **WHEN** user runs `tala send "hello"` with no active session and exactly one open session on the daemon
- **THEN** the message "hello" SHALL be sent to that session
- **AND** the command SHALL print a note naming the target session on stderr

#### Scenario: Send with multiple open sessions lists them

- **WHEN** user runs `tala send "hello"` with no active session and multiple open sessions on the daemon
- **THEN** the command SHALL list the available sessions with their IDs and names
- **THEN** the command SHALL exit with an error (no message SHALL be sent)

### Requirement: `tala send --message-file -` reads from piped stdin

`tala send --message-file -` SHALL read message content from piped stdin. It SHALL NOT attempt to open a file named `-` on disk.

#### Scenario: Send message from piped stdin via --message-file -

- **WHEN** user runs `echo "hello" | tala send --message-file -`
- **THEN** the message "hello" is sent to the current session

#### Scenario: Send message from piped stdin via --message-file - with explicit session

- **WHEN** user runs `cat message.txt | tala send sess_abc123 --message-file -`
- **THEN** the contents of message.txt are sent to session sess_abc123

#### Scenario: --message-file - with no piped input

- **WHEN** user runs `tala send --message-file -` without piping input
- **THEN** an error message is displayed indicating no piped input
