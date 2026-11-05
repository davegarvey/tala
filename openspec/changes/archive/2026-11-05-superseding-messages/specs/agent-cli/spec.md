# Spec Delta

## MODIFIED Requirements

### Requirement: Input and output contract
Send, reply, post and handoff SHALL accept exactly one positional text, message-file, stdin or ordered typed parts input; piped stdin SHALL work implicitly. Direct `send` SHALL accept `--supersedes=<message-id>` to amend an earlier eligible message. JSON message output SHALL expose nullable `supersedes` and `superseded_by` relationships; human output SHALL identify when a message supersedes or has been superseded by another message. Output SHALL default to one JSON envelope with ok and data or error, including usage failures; `--human` SHALL select text rendering and `--json` SHALL be accepted without effect and hidden from help. List output SHALL default to 50 items, cap at 500, and support incremental reads.

#### Scenario: Invalid JSON input
- **WHEN** an invocation has conflicting content sources
- **THEN** exit is 2 and stderr contains a parseable structured error

#### Scenario: JSON without a flag
- **WHEN** an agent runs `tala inbox` without `--json` or `--human`
- **THEN** stdout is a single parseable JSON envelope

#### Scenario: Human rendering on request
- **WHEN** an agent runs `tala inbox --human`
- **THEN** stdout is the text rendering

#### Scenario: Send superseding message
- **WHEN** an agent runs `tala send --to=<recipient> --supersedes=<message-id> <text>`
- **THEN** the result identifies the target in `message.supersedes`
- **AND** the predecessor's `superseded_by` identifies the new message

#### Scenario: Superseding message in human output
- **WHEN** an agent renders an amended message or its predecessor with `--human`
- **THEN** output states which message supersedes which

### Requirement: Actionable errors
Every error SHALL include `code`, `message`, `hint`, a `retry` class of `safe`, `same_key`, `after_fix` or `never`, a `stored` message ID when a message was stored before the failure (otherwise null), and a `next` field holding one exact command or null. The set of error codes SHALL be documented in help output.

#### Scenario: Transport failure before storage
- **WHEN** a send fails with `CONNECTION_ERROR` before the daemon confirms storage
- **THEN** the error has `retry` of `same_key`, `stored` null, and a `next` command that repeats the send with its key

#### Scenario: Daemon unreachable
- **WHEN** a command fails with `DAEMON_UNREACHABLE`
- **THEN** the error has `retry` of `same_key`, `stored` null, and `next` of `tala status`

#### Scenario: Invalid input
- **WHEN** a send names an unknown recipient
- **THEN** the error has `retry` of `after_fix` and `next` lists agents

#### Scenario: Reply to a superseded request
- **WHEN** an agent runs `tala reply <message-id>` for a request with a successor
- **THEN** Tala returns `REQUEST_SUPERSEDED` with `retry` of `after_fix` and no stored message ID
- **AND** the error names the successor ID and provides one exact command to inspect the thread
