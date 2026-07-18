## ADDED Requirements

### Requirement: Session name persists across daemon restarts
The daemon SHALL persist session names to disk so they survive restarts.

#### Scenario: Rename survives daemon restart
- **WHEN** user renames a session with `tala session rename <id> <newname>`
- **AND** daemon is restarted
- **AND** user runs `tala list`
- **THEN** the session name SHALL be `<newname>`

#### Scenario: Session name not overwritten by counterparty message
- **WHEN** user renames a session to `<customname>`
- **AND** counterparty sends a message in that session
- **THEN** the session name SHALL remain `<customname>`

### Requirement: Sessions file format
The daemon SHALL store session names in `{TALA_HOME}/sessions.json` as a JSON object mapping session ID to name string.

#### Scenario: Sessions file written on rename
- **WHEN** user renames a session
- **THEN** `{TALA_HOME}/sessions.json` SHALL contain `{"<session_id>": "<newname>"}`

#### Scenario: Sessions file loaded on daemon start
- **WHEN** daemon starts
- **AND** `{TALA_HOME}/sessions.json` exists
- **THEN** the daemon SHALL load session names from it
