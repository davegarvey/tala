## ADDED Requirements

### Requirement: Wait for next message across all sessions
The system SHALL provide a daemon endpoint `/api/sessions/wait-all` that subscribes to the global event broadcast and returns the next message from any session. It SHALL respond with a `WaitResponse` JSON body.

#### Scenario: Message arrives in any session
- **WHEN** a message is sent to any open session
- **THEN** the wait-all endpoint returns a `WaitResponse` containing that message

#### Scenario: Timeout with no messages
- **WHEN** no message arrives within the `timeout_secs` period
- **THEN** the endpoint returns a `WaitResponse` with `timeout: true`

#### Scenario: Daemon shutting down
- **WHEN** the daemon begins shutdown while waiting
- **THEN** the endpoint returns a `WaitResponse` with `closed: true`

### Requirement: CLI invokes wait-all when no session targeted
`chit wait` without `--session` or active session, when 2+ open sessions exist on the daemon, SHALL call `/api/sessions/wait-all` and display the first received message.

#### Scenario: No explicit session, multiple open sessions
- **WHEN** `chit wait` runs with 2+ open sessions and no `--session` or active session set
- **THEN** it SHALL print a status message to stderr and wait for the next message from any session
- **THEN** it SHALL print the received message with its session ID prefix
- **THEN** it SHALL set the active session to the session the message arrived in
