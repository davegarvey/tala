## REMOVED Requirements

### Requirement: `tala agents` lists unique senders across open sessions

The system SHALL provide a `tala agents` command that lists all unique sender names across all open sessions, grouped by sender, showing last-seen timestamp and message count. Closed sessions SHALL be excluded.

#### Scenario: agents with messages

- **WHEN** user runs `tala agents` and messages exist in open sessions
- **THEN** the system SHALL display a table of unique senders with their sender name, last activity time, and message count

#### Scenario: agents with no messages

- **WHEN** user runs `tala agents` and there are no messages in any open session
- **THEN** the system SHALL display a message like "No active agents found. Start a session with `tala send`, or try `tala discover` to find agents in other projects."

#### Scenario: agents with closed sessions only

- **WHEN** user runs `tala agents` and all sessions are closed
- **THEN** the system SHALL display the "No active agents found" message

#### Scenario: agents --json

- **WHEN** user runs `tala agents --json`
- **THEN** the system SHALL output a JSON array with elements of shape `{"sender": "...", "last_seen": "<ISO-8601>", "message_count": N}`

**Reason**: The local sender aggregation command is outside the intentionally reduced public CLI surface.

**Migration**: Use `tala discover` for cross-project discovery, and use `tala list` or session history for local session inspection. The daemon `/api/agents` endpoint remains available to `tala discover`.
