## Purpose

Session lifecycle in tala: creating a session never silently hijacks the active session, initial messages are stored exactly once, and the active session is set only by explicit `use`.

## ADDED Requirements

### Requirement: `tala start` creates session without setting active

`tala start` SHALL create a new session and print its ID. It SHALL NOT call `write_active_session()`. The new session SHALL NOT become the active session as a side effect.

#### Scenario: Start creates session, no side effect
- **WHEN** user runs `tala start "hello"` while another session is active
- **THEN** a new session is created with the initial message
- **THEN** the active session SHALL remain unchanged

#### Scenario: Start prints session ID
- **WHEN** user runs `tala start`
- **THEN** the session ID is printed to stdout

### Requirement: `tala start` does not duplicate the initial message

`tala start "message"` SHALL store the initial message exactly once. The message SHALL NOT be sent twice.

#### Scenario: Initial message stored once
- **WHEN** user runs `tala start "hello"`
- **THEN** a single message "hello" exists in the session
- **THEN** `tala recap` SHALL show exactly one message

### Requirement: `tala use` sets active session explicitly

`tala use <id>` SHALL set the active session for the current project directory. `tala use --clear` SHALL clear it.

#### Scenario: Use then send without --session
- **WHEN** user runs `tala use sess_abc`
- **THEN** `tala use` SHALL confirm with "Active session set to sess_abc"
- **WHEN** user runs `tala send "message"` (no `--session`)
- **THEN** the message SHALL be sent to sess_abc
