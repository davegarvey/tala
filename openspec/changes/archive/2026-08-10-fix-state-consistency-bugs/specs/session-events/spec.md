## ADDED Requirements

### Requirement: SessionRenamed event broadcast
When a session is renamed, the system SHALL broadcast a `SessionRenamed` event to all SSE consumers.

#### Scenario: Rename broadcasts to SSE listeners
- **WHEN** a user renames a session via `tala rename <session-id> <new-name>`
- **THEN** the daemon SHALL emit a `DaemonEvent::SessionRenamed` containing the session ID, old name, and new name
- **AND** all clients connected via SSE (`tala listen`, `tala stream`) SHALL receive the event

#### Scenario: `tala list` reflects rename after event
- **WHEN** a `SessionRenamed` event is emitted
- **THEN** subsequent calls to `tala list` SHALL show the new session name

### Requirement: SessionRenamed event data fields
The `SessionRenamed` event SHALL contain `id: String`, `old_name: String`, and `new_name: String` fields.

#### Scenario: Event contains all rename metadata
- **WHEN** the daemon emits a `SessionRenamed` event
- **THEN** the event payload SHALL include the session ID, the name before rename, and the name after rename
