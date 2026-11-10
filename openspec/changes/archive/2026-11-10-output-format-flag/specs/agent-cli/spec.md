# Spec Delta

## MODIFIED Requirements

### Requirement: Input and output contract
Send, reply, post and handoff SHALL accept exactly one positional text, message-file, stdin or ordered typed parts input; piped stdin SHALL work implicitly. Direct `send` SHALL accept `--supersedes=<message-id>` to amend an earlier eligible message. JSON message output SHALL expose nullable `supersedes` and `superseded_by` relationships; human output SHALL identify when a message supersedes or has been superseded by another message. Output SHALL default to one JSON envelope with ok and data or error, including usage failures. The global `--format=json|text` option SHALL select the rendering, with `json` as the default; `--human` SHALL remain a hidden alias for `--format=text`, and `--json` SHALL be accepted without effect and hidden from help. List output SHALL default to 50 items, cap at 500, and support incremental reads.

#### Scenario: Invalid JSON input
- **WHEN** an invocation has conflicting content sources
- **THEN** exit is 2 and stderr contains a parseable structured error

#### Scenario: JSON without a flag
- **WHEN** an agent runs `tala inbox` without `--format`, `--json` or `--human`
- **THEN** stdout is a single parseable JSON envelope

#### Scenario: Text rendering on request
- **WHEN** an agent runs `tala inbox --format=text`
- **THEN** stdout is the text rendering

#### Scenario: Human rendering on request
- **WHEN** an agent runs `tala inbox --human`
- **THEN** stdout is the same text rendering as `--format=text`
- **AND** help lists `--format` and does not list `--human` or `--json`

#### Scenario: Conflicting format selection
- **WHEN** an agent passes both `--human` and `--format`
- **THEN** exit is 2 with a structured usage error

#### Scenario: Send superseding message
- **WHEN** an agent runs `tala send --to=<recipient> --supersedes=<message-id> <text>`
- **THEN** the result identifies the target in `message.supersedes`
- **AND** the predecessor's `superseded_by` identifies the new message

#### Scenario: Superseding message in human output
- **WHEN** an agent renders an amended message or its predecessor with `--human`
- **THEN** output states which message supersedes which
