## ADDED Requirements

### Requirement: Close SHALL warn when clearing implicit active session
When closing a session that matches the active session, `tala close` SHALL clear the active session file and print a warning that the active session was closed. If an explicit session ID was provided, no warning is needed.

#### Scenario: close active session without explicit arg
- **GIVEN** active session is `sess_abc`
- **WHEN** user runs `tala close` (no session arg)
- **THEN** the session SHALL be closed
- **AND** the active session SHALL be cleared
- **AND** a warning SHALL be printed: "Active session was closed. Use `tala use <session-id>` to set a new one."

#### Scenario: close active session with explicit args
- **GIVEN** active session is `sess_abc`
- **WHEN** user runs `tala close sess_abc`
- **THEN** the session SHALL be closed
- **AND** the active session SHALL NOT be cleared (user explicitly referenced it)
- **AND** no additional warning beyond normal close output

#### Scenario: close non-active session
- **GIVEN** active session is `sess_abc`
- **WHEN** user runs `tala close sess_def`
- **THEN** the session SHALL be closed
- **AND** the active session SHALL NOT be changed

### Requirement: Reopen SHALL set as active session
When reopening a closed session via `tala session reopen`, the system SHALL set the reopened session as the active session by writing it to `.tala/active-session`. This prevents the "silent switch" problem where the user reopens a session but the active session remains stale.

#### Scenario: reopen sets active
- **GIVEN** active session is `sess_abc` (closed)
- **WHEN** user runs `tala session reopen sess_abc`
- **THEN** the session SHALL be reopened
- **AND** the active session SHALL be set to `sess_abc`
- **AND** a message SHALL be printed: "Reopened session sess_abc (now active)"

#### Scenario: reopen --json
- **WHEN** user runs `tala session reopen sess_abc --json`
- **THEN** the output SHALL include: `{"session_id": "sess_abc", "status": "reopened", "active": true}`
