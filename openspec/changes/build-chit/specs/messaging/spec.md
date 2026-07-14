## ADDED Requirements

### Requirement: Send message
The system SHALL allow agents to send messages in markdown format.

#### Scenario: Send message to a session
- **WHEN** user runs `chit send <session> "<markdown content>"`
- **THEN** the message SHALL be stored in the session
- **THEN** the message SHALL be tagged with the sender's identity (project name or `--as` override)

#### Scenario: Send blocks for reply by default
- **WHEN** user runs `chit send <session> "<content>"`
- **THEN** after sending, the CLI SHALL block and wait for the next message in the session
- **THEN** when a reply arrives, the CLI SHALL print the reply and return

#### Scenario: Send with fire-and-forget flag
- **WHEN** user runs `chit send --ff <session> "<content>"`
- **THEN** the CLI SHALL send the message and return immediately without waiting for a reply

#### Scenario: Send without session auto-targets single session
- **WHEN** user runs `chit send "<content>"` and exactly one session exists
- **THEN** the CLI SHALL send the message to that session
- **WHEN** user runs `chit send "<content>"` and multiple sessions exist
- **THEN** the CLI SHALL error with a list of available sessions

#### Scenario: Send with custom identity
- **WHEN** user runs `chit send <session> "<content>" --as "ci-bot"`
- **THEN** the message SHALL be attributed to "ci-bot" instead of the default project name

### Requirement: Wait for messages
The system SHALL support blocking long-poll to wait for new messages.

#### Scenario: Wait returns new message
- **WHEN** user runs `chit wait <session>` and a new message arrives within the timeout
- **THEN** the CLI SHALL print the message sender, content, and timestamp
- **THEN** the CLI SHALL return with exit code 0

#### Scenario: Wait times out
- **WHEN** user runs `chit wait <session>` and no new message arrives within the timeout
- **THEN** the CLI SHALL print "timeout after <N>s, no new messages"
- **THEN** the CLI SHALL return with exit code 0

#### Scenario: Wait with custom timeout
- **WHEN** user runs `chit wait <session> --timeout 60`
- **THEN** the CLI SHALL block for up to 60 seconds

#### Scenario: Wait without session auto-targets single session
- **WHEN** user runs `chit wait` and exactly one session exists
- **THEN** the CLI SHALL wait on that session
- **WHEN** user runs `chit wait` and multiple sessions exist
- **THEN** the CLI SHALL error with a list of available sessions
- **WHEN** user runs `chit wait` and no sessions exist
- **THEN** the CLI SHALL error with "No active sessions. Start one with `chit start`"

#### Scenario: Wait before any message sent
- **WHEN** user runs `chit wait <session>` and no messages have been sent in that session
- **THEN** the CLI SHALL block until a message arrives or timeout expires

### Requirement: Session closed during wait
The system SHALL notify waiters when a session is closed.

#### Scenario: Wait receives session closed notification
- **WHEN** user is blocked on `chit wait <session>` and another agent closes the session
- **THEN** the wait SHALL return with "session closed" message

### Requirement: Recap full conversation
The system SHALL return the full message transcript of a session.

#### Scenario: Recap returns all messages
- **WHEN** user runs `chit recap <session>`
- **THEN** the CLI SHALL print all messages in chronological order
- **THEN** each message SHALL show sender, timestamp, and content

#### Scenario: Recap with sender attribution
- **WHEN** user runs `chit recap <session>`
- **THEN** each message SHALL be prefixed with the sender identity (e.g., `grubble: "message"`)
