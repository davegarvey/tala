## Purpose

Wait behavior in tala: `wait` adapts to the number of open sessions, sets the active session when messages arrive, and shows which session each message came from.

## ADDED Requirements

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
