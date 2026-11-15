# Spec Delta

## MODIFIED Requirements

### Requirement: Complete clean command surface
The system SHALL expose init, register, update-purpose, rename, unregister, whoami, agents, project (with list, close and reopen), send, inbox, wait, reply, accept, decline, complete, post, board, history, search, pending, resolve, handoff, guide, status, stop, network (with open, extend, close, rotate-token, status, connect and disconnect), and SHALL return the attention summary when run with no subcommand. Retired session commands and aliases, including `list-projects`, `close-project`, `reopen-project`, `enable-network`, `disable-network`, `connect-remote` and `disconnect-remote`, SHALL be absent.

#### Scenario: Accept or decline through the CLI
- **WHEN** the recipient runs `tala accept <id>` or `tala decline <id>`
- **THEN** Tala returns the recorded request decision and optional note or reason

#### Scenario: Complete a request with a prior answer
- **WHEN** the addressed recipient runs `tala complete <request-id> --answered-by=<message-id>` for a valid answer
- **THEN** Tala records completion and returns the request with its linked answer

#### Scenario: Clean help
- **WHEN** an agent runs --help
- **THEN** the new surface is listed and use/session/check/listen are absent
- **AND** rename is listed
- **AND** network commands appear only under `tala network`
- **AND** help points agents to `tala guide` for the quickstart
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

#### Scenario: Read the quickstart
- **WHEN** an agent runs `tala guide` without a format option
- **THEN** Tala prints plain Markdown of fewer than 2,000 Unicode characters covering registration, identity selection, peer discovery, send, inbox, reply and wait
- **AND** it explains private handles versus public IDs, inbox acknowledgement, request replies, listening between work and checking `tala` at work boundaries
- **AND** it lists available topics with one-line descriptions and points to command help for syntax details
- **AND** it requires no selected identity or daemon and creates no state

#### Scenario: Read the complete guide
- **WHEN** an agent runs `tala guide all`
- **THEN** Tala prints the quickstart and all reference topics as plain Markdown

#### Scenario: Read a guide topic
- **WHEN** an agent runs `tala guide listening`, `requests`, `identity`, `worktrees`, `errors` or `harness`
- **THEN** Tala prints only the requested topic as plain Markdown
- **AND** the harness supervisor recipe appears in `harness`, not `listening`

#### Scenario: Explicit guide formats
- **WHEN** an agent runs `tala guide [topic] --format=json`
- **THEN** Tala returns the selected content, topic slug and available topics in the standard JSON success envelope
- **AND** bare guide uses the slug `quickstart`, while `guide all` uses `all`

#### Scenario: Human-readable guide
- **WHEN** an agent runs `tala guide [topic] --format=text`
- **THEN** Tala prints the same Markdown content as the default rendering

#### Scenario: Unknown guide topic
- **WHEN** an agent runs `tala guide unknown`
- **THEN** Tala returns a structured usage error listing valid topics and exits with code 2

#### Scenario: Register points to guide
- **WHEN** an agent registers successfully
- **THEN** registration output includes `data.next` with `tala guide listening`
- **AND** human-readable registration output prints that next command without repeating the private handle

### Requirement: Input and output contract
Send, reply, post and handoff SHALL accept exactly one positional text, message-file, stdin or ordered typed parts input; piped stdin SHALL work implicitly. Direct `send` SHALL accept `--supersedes=<message-id>` to amend an earlier eligible message. JSON message output SHALL expose nullable `supersedes` and `superseded_by` relationships; human output SHALL identify when a message supersedes or has been superseded by another message. Except for successful guide output, output SHALL default to one JSON envelope with ok and data or error, including usage failures. The global `--format=json|text` option SHALL select the rendering, with `json` as the default except that successful `guide` output defaults to text. Explicit format selection SHALL override the guide default before or after its subcommand. Usage errors SHALL retain the existing JSON default unless text is explicitly requested. No other output-selection flags SHALL exist. List output SHALL default to 50 items, cap at 500, and support incremental reads.

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

#### Scenario: Guide format before subcommand
- **WHEN** an agent runs `tala --format=json guide` or `tala --format=text guide all`
- **THEN** the explicit format selects JSON or plain Markdown respectively
