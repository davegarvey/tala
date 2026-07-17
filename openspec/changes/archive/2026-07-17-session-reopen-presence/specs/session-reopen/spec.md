## ADDED Requirements

### Requirement: `chit session reopen` reopens a closed session
`chit session reopen <id>` SHALL set a closed session's `closed` field to `false` and update its `last_activity` timestamp. After reopening, the session SHALL accept new messages via `chit send` or the API. Messages sent after reopening SHALL receive sequential IDs continuing from the session's existing message sequence. The daemon SHALL broadcast a `DaemonEvent::SessionReopened` to global and per-session subscribers.

#### Scenario: Reopen a closed session
- **WHEN** user runs `chit session reopen sess_abc` on a closed session
- **THEN** the session SHALL be reopened (closed = false)
- **THEN** stdout SHALL contain "Session sess_abc reopened"

#### Scenario: Send to reopened session
- **WHEN** a session is closed, then reopened via `chit session reopen`
- **AND** user runs `chit send --session sess_abc "new message"`
- **THEN** the message SHALL be accepted and stored in the session

#### Scenario: Reopen an already-open session
- **WHEN** user runs `chit session reopen sess_abc` on a session that is already open (closed = false)
- **THEN** the command SHALL succeed silently (no error, no state change, no event broadcast)

#### Scenario: Reopen a non-existent session
- **WHEN** user runs `chit session reopen nonexistent`
- **THEN** the command SHALL error with "Session 'nonexistent' not found"

### Requirement: `chit session reopen --json` output
`chit session reopen` SHALL accept a `--json` / `-j` flag. When set, the response SHALL be a JSON object with `session_id` and `status` fields.

#### Scenario: Reopen with --json
- **WHEN** user runs `chit session reopen sess_abc --json` on a closed session
- **THEN** stdout SHALL contain `{"session_id": "sess_abc", "status": "reopened"}`
- **THEN** the human-readable "Session sess_abc reopened" SHALL NOT appear on stdout
