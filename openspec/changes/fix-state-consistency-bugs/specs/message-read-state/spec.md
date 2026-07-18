## ADDED Requirements

### Requirement: Recap marks messages as read
When a user views a session transcript via `tala recap`, the system SHALL mark all messages in that session as read.

#### Scenario: recap clears unread indicator
- **WHEN** a user runs `tala recap <session-id>`
- **THEN** the system SHALL call `write_cursor()` after displaying the transcript
- **AND** subsequent calls to `tala list` and `tala status` SHALL NOT show unread indicators for that session

#### Scenario: Recap with specific message count
- **WHEN** a user runs `tala recap <session-id> --messages 5`
- **THEN** the system SHALL mark only the displayed messages as read
