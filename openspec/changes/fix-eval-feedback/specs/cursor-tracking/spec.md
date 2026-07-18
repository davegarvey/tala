## ADDED Requirements

### Requirement: wait updates cursor after receiving messages
`tala wait` SHALL persist the cursor to `.tala/cursor` after receiving new messages, using the highest message ID from the response.

#### Scenario: cursor updated after wait receives messages
- **WHEN** `tala wait` receives one or more new messages
- **THEN** tala writes the maximum message ID from those messages to the cursor file
- **THEN** a subsequent `tala whatsup` call correctly reports those messages as already seen

#### Scenario: wait does not update cursor on timeout
- **WHEN** `tala wait` times out with no new messages
- **THEN** the cursor SHALL NOT be modified

#### Scenario: wait does not update cursor on closed session
- **WHEN** `tala wait` detects the session is closed
- **THEN** the cursor SHALL NOT be modified

### Requirement: cursor update does not break other commands
Adding cursor updates to `tala wait` SHALL NOT change the behavior of `tala send`, `tala listen`, or `tala whatsup`.

#### Scenario: send still updates cursor
- **WHEN** user sends a message with `tala send`
- **THEN** the cursor is still updated as before

#### Scenario: listen still updates cursor
- **WHEN** user receives messages with `tala listen`
- **THEN** the cursor is still updated as before
