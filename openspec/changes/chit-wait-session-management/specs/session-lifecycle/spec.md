## ADDED Requirements

### Requirement: `chit start` creates session without setting active
`chit start` SHALL create a new session and print its ID. It SHALL NOT call `write_active_session()`. The new session SHALL NOT become the active session as a side effect.

#### Scenario: Start creates session, no side effect
- **WHEN** user runs `chit start "hello"` while another session is active
- **THEN** a new session is created with the initial message
- **THEN** the active session SHALL remain unchanged

#### Scenario: Start prints session ID
- **WHEN** user runs `chit start`
- **THEN** the session ID is printed to stdout

### Requirement: `chit start` does not duplicate the initial message
`chit start "message"` SHALL store the initial message exactly once. The message SHALL NOT be sent twice.

#### Scenario: Initial message stored once
- **WHEN** user runs `chit start "hello"`
- **THEN** a single message "hello" exists in the session
- **THEN** `chit recap` SHALL show exactly one message

### Requirement: `chit use` sets active session explicitly
`chit use <id>` SHALL set the active session for the current project directory. `chit use --clear` SHALL clear it.

#### Scenario: Use then send without --session
- **WHEN** user runs `chit use sess_abc`
- **THEN** `chit use` SHALL confirm with "Active session set to sess_abc"
- **WHEN** user runs `chit send "message"` (no `--session`)
- **THEN** the message SHALL be sent to sess_abc
