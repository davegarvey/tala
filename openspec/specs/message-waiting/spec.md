## Purpose

Wait behavior in tala: `wait` adapts to the number of open sessions, sets the active session when messages arrive, and shows which session each message came from.

## Requirements

### Requirement: `tala wait` adapts to session count

`tala wait` without `--session` or active session SHALL resolve the target session based on the daemon state:
- 0 open sessions → call `/api/sessions/wait-new`, block until a new session is created
- 1 open session → wait on that session for new messages
- 2+ open sessions → call `/api/sessions/wait-all`, return the next message from any session

#### Scenario: No sessions, wait for new
- **WHEN** `tala wait` runs with 0 open sessions
- **THEN** it SHALL print "No active sessions. Waiting for a new session..." to stderr
- **THEN** it SHALL block until another agent creates a session
- **THEN** it SHALL print "New session: <id>" to stderr
- **THEN** it SHALL wait for and display messages in that session

#### Scenario: One session, wait on it
- **WHEN** `tala wait` runs with exactly 1 open session
- **THEN** it SHALL print "Waiting for new messages in session <id>..." to stderr
- **THEN** it SHALL wait for new messages in that session

#### Scenario: Multiple sessions, wait-all
- **WHEN** `tala wait` runs with 2+ open sessions
- **THEN** it SHALL print "Waiting for new messages from any session..." to stderr
- **THEN** it SHALL wait for the next message from any session
- **THEN** it SHALL display the message with its session ID

#### Scenario: Explicit --session still works
- **WHEN** user runs `tala wait --session sess_abc`
- **THEN** it SHALL wait on sess_abc regardless of how many sessions exist

#### Scenario: Active session takes priority
- **WHEN** user runs `tala wait` with an active session set
- **THEN** it SHALL wait on that session regardless of how many sessions exist on the daemon

### Requirement: `tala wait` sets active session on receipt

When `tala wait` receives messages, it SHALL call `write_active_session()` so subsequent `tala send` (without `--session`) targets the same session.

#### Scenario: Wait then send without --session
- **WHEN** agent runs `tala wait` and receives a message from session sess_abc
- **THEN** active session is set to sess_abc
- **WHEN** agent runs `tala send "reply"`
- **THEN** the reply SHALL be sent to sess_abc

### Requirement: `tala wait` shows session ID in output

When displaying received messages, `tala wait` SHALL prefix each message with `[sess <id>]` so the session is visible.

#### Scenario: Message display includes session
- **WHEN** agent receives a message via `tala wait`
- **THEN** the output SHALL be in format `[sess <id>] [<msg_id>] <sender> (<time>):\n    <content>`

### Requirement: `tala wait` handles a stale active session gracefully

When `tala wait` receives a `SESSION_NOT_FOUND` error from the daemon for the active session, it SHALL NOT hard-error. Instead, it SHALL clear the stale active session, print a note that the active session was stale, and re-resolve the wait target from scratch — falling back through the same session-count logic as the no-active-session path.

#### Scenario: Stale active session, one active session exists

- **GIVEN** `.tala/active-session` contains a stale session ID `sess_stale`
- **AND** the daemon has one active session `sess_abc`
- **WHEN** user runs `tala wait`
- **THEN** the stale active session is cleared
- **THEN** `tala wait` uses `sess_abc` and waits for new messages

#### Scenario: Stale active session, no active sessions exist

- **GIVEN** `.tala/active-session` contains a stale session ID `sess_stale`
- **AND** the daemon has no active sessions (all closed or none)
- **WHEN** user runs `tala wait`
- **THEN** the stale active session is cleared
- **THEN** `tala wait` SHALL wait for a new session

#### Scenario: Stale active session, multiple active sessions exist

- **GIVEN** `.tala/active-session` contains a stale session ID `sess_stale`
- **AND** the daemon has multiple active sessions (`sess_abc`, `sess_def`)
- **WHEN** user runs `tala wait`
- **THEN** the stale active session is cleared
- **THEN** `tala wait` SHALL resolve the target via the standard multi-session path (same as no active session being set)

### Requirement: `tala wait` timeout feedback

When `tala wait` times out with no new messages, the CLI SHALL print a message of the form "timeout after <N>s, no new messages" and SHALL exit with code 3 (the benign-timeout exit code, distinct from usage errors). `tala wait --timeout <N>` SHALL override the default wait timeout (in seconds).

#### Scenario: Wait times out

- **WHEN** user runs `tala wait <session>` and no new message arrives within the timeout
- **THEN** the CLI SHALL print "timeout after <N>s, no new messages"
- **THEN** the CLI SHALL return with exit code 3

#### Scenario: Wait with custom timeout

- **WHEN** user runs `tala wait <session> --timeout 60`
- **THEN** the CLI SHALL block for up to 60 seconds

### Requirement: `tala wait` notifies when the session closes

The CLI SHALL notify waiters when the session they are waiting on is closed during the wait.

#### Scenario: Wait receives session closed notification

- **WHEN** user is blocked on `tala wait <session>` and another agent closes the session
- **THEN** the wait SHALL return with "[session closed]" message
