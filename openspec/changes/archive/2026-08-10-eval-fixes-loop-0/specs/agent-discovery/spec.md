## ADDED Requirements

### Requirement: chit agents SHALL list unique senders across all open sessions
The system SHALL provide a `chit agents` command that lists all unique sender names across all open sessions, grouped by sender, showing last-seen timestamp and number of sessions. Closed sessions SHALL be excluded.

#### Scenario: chit agents with messages
- **WHEN** user runs `chit agents`
- **THEN** the system SHALL display a table of unique senders with their sender name, last activity time, and message count

#### Scenario: chit agents with no messages
- **WHEN** user runs `chit agents` and there are no messages in any open session
- **THEN** the system SHALL display a message like "No active agents found. Start a session with `chit start <message>`."

#### Scenario: chit agents --json
- **WHEN** user runs `chit agents --json`
- **THEN** the system SHALL output a JSON array with elements of shape: `{"sender": "...", "last_seen": "...", "message_count": N}`

#### Scenario: chit agents with closed sessions only
- **WHEN** user runs `chit agents` and all sessions are closed
- **THEN** the system SHALL display "No active agents found."

### Requirement: AgentSummary model
The system SHALL define an `AgentSummary` struct in models.rs with fields: `sender: String`, `last_seen: DateTime<Utc>`, `message_count: usize`.

#### Scenario: AgentSummary serialization
- **WHEN** `chit agents --json` is used
- **THEN** each agent SHALL be serialized as `{"sender": "...", "last_seen": "<ISO-8601>", "message_count": N}`

### Requirement: Daemon endpoint for agents
The system SHALL add a `GET /api/agents` endpoint that iterates open sessions and aggregates unique sender stats.

#### Scenario: GET /api/agents
- **WHEN** daemon receives `GET /api/agents`
- **THEN** it SHALL return a JSON array of `AgentSummary` objects derived from messages in open sessions

### Requirement: chit observe/listen SHALL mention agent discovery
The help text for `listen` (formerly observe) SHALL mention that users can run `chit agents` to see who's active.

#### Scenario: listen help mentions agents
- **WHEN** user runs `chit listen --help`
- **THEN** the help text SHALL mention `chit agents` as a way to discover active participants
