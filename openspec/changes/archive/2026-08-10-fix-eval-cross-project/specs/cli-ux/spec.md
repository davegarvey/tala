## ADDED Requirements

### Requirement: Delivery indication on tala start without --wait
When `tala start` runs without `--wait`, the system SHALL indicate whether any agents are actively listening in the session, so the user knows their message was queued.

#### Scenario: No agents listening
- **WHEN** user runs `tala start "hello"` without `--wait`
- **AND** no other agents are connected to the session
- **THEN** the output SHALL include an indication that no agents are currently listening

#### Scenario: Agents are listening
- **WHEN** user runs `tala start "hello"` without `--wait`
- **AND** other agents are connected to the session
- **THEN** the output SHALL indicate that agents are listening

### Requirement: --file renamed to --message-file on tala send
The `--file` flag on `tala send` SHALL be renamed to `--message-file` to clarify that it reads message content from a file rather than attaching a file. The old `--file` flag SHALL continue to work with a deprecation warning.

#### Scenario: New flag works
- **WHEN** user runs `tala send --message-file /tmp/msg.txt`
- **THEN** the message content SHALL be read from `/tmp/msg.txt` and sent

#### Scenario: Old flag still works with warning
- **WHEN** user runs `tala send --file /tmp/msg.txt`
- **THEN** the message content SHALL be read from `/tmp/msg.txt` and sent
- **AND** a deprecation warning SHALL be printed to stderr

#### Scenario: Help text shows new flag
- **WHEN** user runs `tala send --help`
- **THEN** the help text SHALL show `--message-file` instead of `--file`

### Requirement: --new-session surfaced in top-level help
The top-level `tala --help` output SHALL mention `tala wait --new-session` as a way to wait for a new session to be created.

#### Scenario: Top-level help mentions --new-session
- **WHEN** user runs `tala --help`
- **THEN** the output SHALL include a reference to `tala wait --new-session`
