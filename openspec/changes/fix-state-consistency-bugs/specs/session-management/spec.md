## ADDED Requirements

### Requirement: Rename triggers SessionRenamed event
When a session is renamed, the session management system SHALL emit a `SessionRenamed` event through the daemon's event system.

#### Scenario: Rename operation emits event
- **WHEN** the rename operation completes successfully
- **THEN** the system SHALL construct a `DaemonEvent::SessionRenamed` with the session ID, old name, and new name
- **AND** the system SHALL broadcast the event to all connected SSE clients
