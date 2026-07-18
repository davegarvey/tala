## ADDED Requirements

### Requirement: Wait --new-session shows initial feedback
When `tala wait --new-session` begins waiting, the system SHALL print an immediate message to stderr confirming it is waiting and showing the timeout value.

#### Scenario: Wait --new with feedback
- **WHEN** user runs `tala wait --new-session --timeout 30`
- **THEN** the system SHALL print `Waiting for a new session (timeout: 30s)...` to stderr before initiating the long poll

### Requirement: Listen help text mentions --since for skipping history
The `tala listen` help text SHALL document the `--since` flag as a way to skip historical message replay. A dedicated `--new-only` flag is deferred to a future loop as it requires server-side changes (the `since` parameter controls both history replay AND new message filtering in the SSE endpoint, so a client-only flag cannot reliably skip history without also suppressing new messages).

#### Scenario: Listen help shows --since
- **WHEN** user runs `tala listen --help`
- **THEN** the help output SHALL mention `--since` as a way to skip historical message replay

### Requirement: Use without args lists sessions
When `tala use` is run without arguments and no active session is set, the system SHALL list available sessions instead of just saying "no active session".

#### Scenario: Use with no active session
- **WHEN** no active session is set
- **AND** user runs `tala use`
- **THEN** the system SHALL display a list of available sessions with their IDs, names, and message counts

### Requirement: CLI help cross-references wait/stream/listen
The `--help` output for `tala wait`, `tala stream`, and `tala listen` SHALL include brief usage guidance explaining when to use each command.

#### Scenario: Wait help shows cross-references
- **WHEN** user runs `tala wait --help`
- **THEN** the help output SHALL mention `tala stream` for real-time SSE and `tala listen` for observing all sessions

#### Scenario: Stream help shows cross-references
- **WHEN** user runs `tala stream --help`
- **THEN** the help output SHALL mention `tala wait` for blocking poll and `tala listen` for observing all sessions

#### Scenario: Listen help shows cross-references
- **WHEN** user runs `tala listen --help`
- **THEN** the help output SHALL mention `tala stream` for single-session SSE and `tala wait` for blocking poll

### Requirement: Wait --new-session mentioned in top-level help
The `tala wait` command doc comment (shown in `tala --help`) SHALL mention `--new-session` as a usage option.

#### Scenario: Wait doc comment includes --new-session
- **WHEN** user runs `tala --help`
- **THEN** the one-line description for `wait` SHALL mention `--new-session`

### Requirement: Cursor updated on send
When `tala send` successfully sends a message, the system SHALL update the local cursor file to include the sent message ID.

#### Scenario: Send updates cursor
- **WHEN** user runs `tala send "hello"`
- **THEN** the cursor file SHALL be updated with the sent message ID
- **AND** subsequent `tala list` SHALL NOT count the sent message as unread
