## ADDED Requirements

### Requirement: Create session
The system SHALL create a new session with an auto-generated ID.

#### Scenario: Start creates session with auto-generated ID
- **WHEN** user runs `chit start`
- **THEN** a new session SHALL be created with an ID in the format `sess_<random>`
- **THEN** the session SHALL be listed in the daemon's active sessions

#### Scenario: Session IDs are unique
- **WHEN** multiple sessions are created
- **THEN** each session SHALL have a unique ID

### Requirement: List sessions
The system SHALL list all active sessions.

#### Scenario: List shows all active sessions
- **WHEN** user runs `chit list` and sessions exist
- **THEN** the CLI SHALL print each session ID, creation time, and message count

#### Scenario: List with no sessions
- **WHEN** user runs `chit list` and no sessions exist
- **THEN** the CLI SHALL print "No active sessions"

### Requirement: Close session
The system SHALL close an active session.

#### Scenario: Close session removes from active list
- **WHEN** user runs `chit close <session>`
- **THEN** the session SHALL be marked as closed
- **THEN** the session SHALL no longer appear in `chit list`

#### Scenario: Close session notifies other agents
- **WHEN** user runs `chit close <session>` and other agents are waiting on that session
- **THEN** the waiting agents SHALL receive a "session closed" notification

#### Scenario: Close session with no session ID auto-targets single session
- **WHEN** user runs `chit close` and exactly one session exists
- **THEN** that session SHALL be closed
- **WHEN** user runs `chit close` and multiple sessions exist
- **THEN** the CLI SHALL error with a list of available sessions

### Requirement: Auto-target single session
When a session ID is optional and exactly one session exists, commands SHALL target that session automatically.

#### Scenario: Single session auto-target applies to send, wait, close, recap
- **WHEN** user runs any of `chit send`, `chit wait`, `chit close`, or `chit recap` without a session ID and exactly one session exists
- **THEN** the command SHALL apply to that session

#### Scenario: No sessions produces clear error
- **WHEN** user runs a session-scoped command without a session ID and no sessions exist
- **THEN** the CLI SHALL print a clear error message directing the user to `chit start`
