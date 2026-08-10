## Purpose

Message read state in tala: viewing a transcript or streaming messages advances the project's per-session read cursor, so `tala list` and `tala status` stop reporting those messages as new.

## Requirements

### Requirement: Recap marks messages as read

When a user views a session transcript via `tala history`, the system SHALL mark the displayed messages as read by persisting the per-session read cursor to `.tala/cursors.json`. The daemon SHALL also record the reader's read receipt for the session.

#### Scenario: Recap clears unread indicator

- **WHEN** a user runs `tala history <session-id>`
- **THEN** the system SHALL write the session's cursor file with the highest displayed message ID
- **AND** subsequent calls to `tala list` and `tala status` SHALL NOT show unread indicators for that session

#### Scenario: Recap with specific message count

- **WHEN** a user runs `tala history <session-id> --limit 5`
- **THEN** the system SHALL mark only the displayed messages as read (the cursor SHALL be the highest ID among those 5)

#### Scenario: Recap of an empty session does not reset read state

- **WHEN** a user runs `tala history` on a session with no messages
- **THEN** the read cursor SHALL NOT be modified

### Requirement: Listen advances per-session read cursors

When `tala listen` receives messages, the system SHALL advance each session's read cursor to the highest message ID seen in that session, so those messages are not re-reported as new.

#### Scenario: Listen updates cursor after receiving messages

- **WHEN** `tala listen` receives one or more messages from a session
- **THEN** the system SHALL write the maximum message ID seen in that session to `.tala/cursors.json`
- **AND** subsequent calls to `tala list` and `tala status` SHALL NOT show those messages as unread

#### Scenario: Listen with no messages does not touch cursors

- **WHEN** `tala listen` connects and receives no messages
- **THEN** the cursors file SHALL NOT be modified
