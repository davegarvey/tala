## ADDED Requirements

### Requirement: Enhanced tala use output
`tala use` with no arguments SHALL display the active session ID, session name, and message count. When setting a session, the output SHALL include the session ID, name, and message count.

#### Scenario: Show active session with details
- **WHEN** user runs `tala use` with an active session set
- **THEN** the output SHALL include the session ID, session name, and total message count

#### Scenario: Set active session with confirmation
- **WHEN** user runs `tala use <session-id>`
- **THEN** the output SHALL include the session ID, session name, and message count

#### Scenario: No active session
- **WHEN** user runs `tala use` with no active session
- **THEN** the output SHALL indicate no active session is set
