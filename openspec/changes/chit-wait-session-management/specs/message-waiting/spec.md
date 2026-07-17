## ADDED Requirements

### Requirement: `chit wait` adapts to session count
`chit wait` without `--session` or active session SHALL resolve the target session based on the daemon state:
- 0 open sessions → call `/api/sessions/wait-new`, block until a new session is created
- 1 open session → wait on that session for new messages
- 2+ open sessions → call `/api/sessions/wait-all`, return the next message from any session

#### Scenario: No sessions, wait for new
- **WHEN** `chit wait` runs with 0 open sessions
- **THEN** it SHALL print "No active sessions. Waiting for a new session..." to stderr
- **THEN** it SHALL block until another agent creates a session
- **THEN** it SHALL print "New session: <id>" to stderr
- **THEN** it SHALL wait for and display messages in that session

#### Scenario: One session, wait on it
- **WHEN** `chit wait` runs with exactly 1 open session
- **THEN** it SHALL print "Waiting for new messages in session <id>..." to stderr
- **THEN** it SHALL wait for new messages in that session

#### Scenario: Multiple sessions, wait-all
- **WHEN** `chit wait` runs with 2+ open sessions
- **THEN** it SHALL print "Waiting for new messages from any session..." to stderr
- **THEN** it SHALL wait for the next message from any session
- **THEN** it SHALL display the message with its session ID

#### Scenario: Explicit --session still works
- **WHEN** user runs `chit wait --session sess_abc`
- **THEN** it SHALL wait on sess_abc regardless of how many sessions exist

#### Scenario: Active session takes priority
- **WHEN** user runs `chit wait` with an active session set
- **THEN** it SHALL wait on that session regardless of how many sessions exist on the daemon

### Requirement: `chit wait` sets active session on receipt
When `chit wait` receives messages, it SHALL call `write_active_session()` so subsequent `chit send` (without `--session`) targets the same session.

#### Scenario: Wait then send without --session
- **WHEN** agent runs `chit wait` and receives a message from session sess_abc
- **THEN** active session is set to sess_abc
- **WHEN** agent runs `chit send "reply"`
- **THEN** the reply SHALL be sent to sess_abc

### Requirement: `chit wait` shows session ID in output
When displaying received messages, `chit wait` SHALL prefix each message with `[sess <id>]` so the session is visible.

#### Scenario: Message display includes session
- **WHEN** agent receives a message via `chit wait`
- **THEN** the output SHALL be in format `[sess <id>] [<msg_id>] <sender> (<time>):\n    <content>`
