## ADDED Requirements

### Requirement: `chit use` on closed session gives actionable error
When `chit use` is given a session ID that exists but is closed, the error message SHALL clearly state the session is closed and suggest using `chit session reopen` to continue it. The CLI SHALL query `GET /api/sessions/:id` to check the session's closed status before setting it as active.

#### Scenario: Use by ID on closed session
- **WHEN** user runs `chit use sess_abc` and `sess_abc` exists but is closed
- **THEN** the command SHALL error with "Session 'sess_abc' is closed. Use \`chit session reopen\` to continue"

#### Scenario: Use by name on closed session
- **WHEN** user runs `chit use my-session` and a session named "my-session" exists but is closed
- **THEN** the command SHALL error with "Session 'my-session' is closed. Use \`chit session reopen\` to continue" (name lookup MAY include closed sessions for this check to provide a better error message; if the name is not found at all, fall through to "No active session named 'my-session'")

#### Scenario: Use by ID on non-existent session
- **WHEN** user runs `chit use nonexistent` and no session with that ID exists
- **THEN** the command SHALL error with existing "session not found" error (unchanged behavior)

### Requirement: `chit close --quiet` suppresses confirmation
`chit close` SHALL accept a `--quiet` / `-q` flag that suppresses the human-readable confirmation message. When `--json` is also set, `--quiet` SHALL NOT suppress the JSON output.

#### Scenario: Close with --quiet (human-readable)
- **WHEN** user runs `chit close sess_abc --quiet`
- **THEN** the session SHALL be closed
- **THEN** no human-readable confirmation SHALL appear on stdout

#### Scenario: Close with --quiet and --json
- **WHEN** user runs `chit close sess_abc --quiet --json`
- **THEN** stdout SHALL contain the JSON response (quiet does not suppress JSON)

#### Scenario: Close without --quiet shows confirmation
- **WHEN** user runs `chit close sess_abc` (no --quiet)
- **THEN** the session SHALL be closed
- **THEN** stdout SHALL contain "Session sess_abc closed" (existing behavior preserved)

### Requirement: `chit stream` alias for `chit follow`
`chit follow` SHALL have `chit stream` registered as a command alias. Both names SHALL invoke the same functionality.

#### Scenario: Stream works as follow alias
- **WHEN** user runs `chit stream --session sess_abc --since 0`
- **THEN** the behavior SHALL be identical to `chit follow --session sess_abc --since 0`
