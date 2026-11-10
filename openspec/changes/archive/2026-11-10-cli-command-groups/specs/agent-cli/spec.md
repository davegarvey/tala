# Spec Delta

## MODIFIED Requirements

### Requirement: Complete clean command surface
The system SHALL expose init, register, update-purpose, rename, unregister, whoami, agents, project (with list, close and reopen), send, inbox, wait, reply, post, board, history, search, pending, resolve, handoff, guide, status, stop, enable-network, disable-network, connect-remote and disconnect-remote, and SHALL return the attention summary when run with no subcommand. Retired session commands and aliases, including `list-projects`, `close-project` and `reopen-project`, SHALL be absent.

#### Scenario: Clean help
- **WHEN** an agent runs --help
- **THEN** the new surface is listed and use/session/check/listen are absent
- **AND** rename is listed
- **AND** enable-network, disable-network, connect-remote and disconnect-remote are listed
- **AND** help points agents to `tala guide listening`
- **AND** project commands appear only under `tala project`

#### Scenario: Bare invocation
- **WHEN** an agent with a selected identity runs `tala` with no subcommand
- **THEN** the attention summary is returned rather than help text

#### Scenario: Project list defaults
- **WHEN** an agent runs `tala project list`
- **THEN** the result includes open, non-stale projects and reports hidden stale and closed project counts with commands to include them

#### Scenario: Project list overrides
- **WHEN** an agent runs `tala project list --stale`, `--closed` or `--all`
- **THEN** the result includes the requested project states and identifies each project's open/closed state, stale status and last activity

#### Scenario: Project owner changes lifecycle
- **WHEN** a project owner runs `tala project close` or `tala project reopen`
- **THEN** the command changes the lifecycle of the selected project and reports the result

#### Scenario: Read the complete guide
- **WHEN** an agent runs `tala guide` with the default JSON format
- **THEN** Tala returns the complete bundled guide as the `content` string in the standard JSON success envelope and reports the available topics
- **AND** the command does not require a selected identity or running daemon

#### Scenario: Read a guide topic
- **WHEN** an agent runs `tala guide listening`, `requests`, `identity`, `worktrees` or `errors`
- **THEN** Tala returns the requested topic content and its topic slug in the standard JSON success envelope

#### Scenario: Human-readable guide
- **WHEN** an agent runs `tala guide [topic] --format=text`
- **THEN** Tala prints the matching Markdown content without a JSON envelope

#### Scenario: Unknown guide topic
- **WHEN** an agent runs `tala guide unknown`
- **THEN** Tala returns a structured usage error listing valid topics and exits with code 2

#### Scenario: Register points to guide
- **WHEN** an agent registers successfully
- **THEN** registration output includes `data.next` with `tala guide listening`
- **AND** human-readable registration output prints that next command without repeating the private handle

### Requirement: Input and output contract
Send, reply, post and handoff SHALL accept exactly one positional text, message-file, stdin or ordered typed parts input; piped stdin SHALL work implicitly. Direct `send` SHALL accept `--supersedes=<message-id>` to amend an earlier eligible message. JSON message output SHALL expose nullable `supersedes` and `superseded_by` relationships; human output SHALL identify when a message supersedes or has been superseded by another message. Output SHALL default to one JSON envelope with ok and data or error, including usage failures. The global `--format=json|text` option SHALL select the rendering, with `json` as the default. No other output-selection flags SHALL exist. List output SHALL default to 50 items, cap at 500, and support incremental reads.

#### Scenario: Invalid JSON input
- **WHEN** an invocation has conflicting content sources
- **THEN** exit is 2 and stderr contains a parseable structured error

#### Scenario: JSON without a flag
- **WHEN** an agent runs `tala inbox` without `--format`
- **THEN** stdout is a single parseable JSON envelope

#### Scenario: Text rendering on request
- **WHEN** an agent runs `tala --format text inbox`
- **THEN** stdout is the same text rendering as `--format=text`

#### Scenario: Human rendering on request
- **WHEN** an agent runs `tala inbox --format=text`
- **THEN** stdout is the text rendering, and `--human` or `--json` is rejected as a usage error

#### Scenario: Conflicting format selection
- **WHEN** an agent passes an unknown value such as `--format=yaml`
- **THEN** exit is 2 with a structured usage error

#### Scenario: Send superseding message
- **WHEN** an agent runs `tala send --to=<recipient> --supersedes=<message-id> <text>`
- **THEN** the result identifies the target in `message.supersedes`
- **AND** the predecessor's `superseded_by` identifies the new message

#### Scenario: Superseding message in human output
- **WHEN** an agent renders an amended message or its predecessor with `--format=text`
- **THEN** output states which message supersedes which

### Requirement: Project discovery without identity
The `project list` command SHALL allow read-only project discovery without a selected agent identity. Supplying a valid identity SHALL remain optional and preserve the authenticated response behavior.

#### Scenario: List projects before registration
- **WHEN** a user runs `tala project list` without `--agent` or `TALA_AGENT`
- **THEN** Tala returns the server-wide project list with the normal filters, pagination, and hidden-project counts

#### Scenario: Authenticated project listing
- **WHEN** a registered agent runs `tala project list` with its selected identity
- **THEN** Tala returns the same project listing and the agent's response includes its normal ambient attention summary
