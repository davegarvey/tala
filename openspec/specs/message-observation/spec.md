## Purpose

Live observation of all sessions in tala: `tala listen` streams every session's messages over SSE, replays history from each session's own read checkpoint by default, and ends cleanly on a server-driven timeout — so agents can watch the whole conversation space without replaying messages they have already seen.

## Requirements

### Requirement: `tala listen` observes all sessions

The system SHALL provide a `listen` command that connects to the daemon's `/api/observe` SSE endpoint and streams messages from all sessions (the renamed successor of `observe`). On connect, the stream SHALL first replay the sessions' history, then deliver new messages as they arrive. `--since` SHALL set a lower id bound for history replay; `--from <sender>`, `--match <text>`, and `--name <text>` SHALL filter messages by sender, text content, and session name respectively. Human-readable output SHALL label each message with its session (name, falling back to id) and show sender and timestamp; `--json` SHALL emit the raw event stream.

#### Scenario: Listen streams new messages
- **WHEN** user runs `tala listen` and a message arrives in any session
- **THEN** the message is printed with its session label, sender, and timestamp

#### Scenario: Listen replays history
- **WHEN** user runs `tala listen --since 0`
- **THEN** the full history of every session is replayed before new messages are streamed

#### Scenario: Listen filters by sender
- **WHEN** user runs `tala listen --from alpha`
- **THEN** only messages from sender "alpha" are shown

### Requirement: `tala listen` defaults to per-session checkpoints

When `tala listen` runs without `--since`, it SHALL replay each session from ITS OWN stored read cursor (the `.tala` cursor map) instead of from id 0, so reconnecting does not replay history the client has already seen. A global `--since 0` SHALL still replay full history. When the stream delivers messages, the CLI SHALL advance the per-session cursors to the highest id seen, matching `tala check` behavior.

#### Scenario: Listen without --since shows only new messages
- **GIVEN** the stored cursor for session `sess_abc` is 42
- **WHEN** user runs `tala listen` (no `--since`)
- **THEN** messages with id ≤ 42 in `sess_abc` SHALL NOT be replayed

#### Scenario: Listen --since 0 replays full history
- **WHEN** user runs `tala listen --since 0`
- **THEN** the system SHALL replay all messages, preserving the full-history behavior

#### Scenario: Cursor advances during listen
- **WHEN** `tala listen` receives messages and the stream closes
- **THEN** the per-session cursors SHALL be updated to the latest message id seen

### Requirement: `tala listen` timeout is server-driven

`tala listen` SHALL accept `--timeout <secs>`. The CLI SHALL pass the value as the `timeout_secs` query parameter on `/api/observe`, and the server SHALL close the SSE stream when the duration expires — racing the broadcast receiver against a sleep — even if no messages have arrived. When `--timeout` is omitted, the stream SHALL run for the default 60 seconds. When the stream ends, the CLI SHALL print a summary line naming the connection and the number of messages received.

#### Scenario: Listen with timeout exits cleanly
- **WHEN** user runs `tala listen --timeout 5`
- **THEN** the stream runs for approximately 5 seconds and the command exits with success

#### Scenario: Server closes the stream after the timeout
- **WHEN** the daemon receives `GET /api/observe?timeout_secs=3`
- **THEN** the SSE stream closes after approximately 3 seconds and the HTTP response completes normally

#### Scenario: Default listen timeout
- **WHEN** user runs `tala listen` without `--timeout`
- **THEN** the stream closes after the default 60 seconds

#### Scenario: End-of-stream summary
- **WHEN** the listen stream closes after receiving 5 messages
- **THEN** the CLI SHALL print a connection-closed summary reporting 5 message(s)
