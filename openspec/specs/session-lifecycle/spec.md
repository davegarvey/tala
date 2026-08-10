## Purpose

Session lifecycle in tala: `session create` creates a session and makes it active, `use` sets the active session explicitly, and the active session is never hijacked silently by message receipt.

## Requirements

### Requirement: `tala session create` creates session and sets active

`tala session create` SHALL create a new session, print its ID, and make it the active session for the current project directory.

#### Scenario: Session create prints ID and sets active
- **WHEN** user runs `tala session create` while another session is active
- **THEN** a new session is created
- **THEN** the session ID is printed to stdout
- **THEN** the new session SHALL be the active session (a bare `tala send` targets it)

#### Scenario: Session create with a name
- **WHEN** user runs `tala session create --name review`
- **THEN** a new session named "review" is created and becomes active

### Requirement: `tala use` sets active session explicitly

`tala use <id>` SHALL set the active session for the current project directory. `tala use --clear` SHALL clear it.

#### Scenario: Use then send without --session
- **WHEN** user runs `tala use sess_abc`
- **THEN** `tala use` SHALL confirm with "Active session set to sess_abc"
- **WHEN** user runs `tala send "message"` (no `--session`)
- **THEN** the message SHALL be sent to sess_abc
