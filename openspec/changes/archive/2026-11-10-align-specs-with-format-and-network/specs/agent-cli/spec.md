# Spec Delta

## MODIFIED Requirements

### Requirement: Complete clean command surface
The system SHALL expose init, register, update-purpose, rename, unregister, whoami, agents, list-projects, close-project, reopen-project, send, inbox, wait, reply, post, board, history, search, pending, resolve, handoff, guide, status, stop, enable-network, disable-network, connect-remote and disconnect-remote, and SHALL return the attention summary when run with no subcommand. Retired session commands and aliases SHALL be absent.

#### Scenario: Clean help
- **WHEN** an agent runs --help
- **THEN** the new surface is listed and use/session/check/listen are absent
- **AND** rename is listed
- **AND** enable-network, disable-network, connect-remote and disconnect-remote are listed
- **AND** help points agents to `tala guide listening`

#### Scenario: Bare invocation
- **WHEN** an agent with a selected identity runs `tala` with no subcommand
- **THEN** the attention summary is returned rather than help text

#### Scenario: Project list defaults
- **WHEN** an agent runs `tala list-projects`
- **THEN** the result includes open, non-stale projects and reports hidden stale and closed project counts with commands to include them

#### Scenario: Project list overrides
- **WHEN** an agent runs `tala list-projects --stale`, `--closed` or `--all`
- **THEN** the result includes the requested project states and identifies each project's open/closed state, stale status and last activity

#### Scenario: Project owner changes lifecycle
- **WHEN** a project owner runs `tala close-project` or `tala reopen-project`
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

### Requirement: Purpose CLI operations and rendering
The CLI SHALL accept optional purpose at registration and expose an identity-bound `update-purpose` command to set or clear it. `whoami` and `agents` SHALL return the same nullable purpose fields in JSON; human output SHALL safely display purpose, escape controls, and truncate at 80 Unicode scalar values.

#### Scenario: Register with a purpose
- **WHEN** an agent registers with `--purpose=<text>`
- **THEN** the returned agent record includes the normalized purpose and update time
- **AND** user-facing help explains that peers can see the purpose and it must not contain secrets, personal data, or instructions

#### Scenario: Set or clear purpose
- **WHEN** an agent runs `tala update-purpose --purpose=<text>` or `tala update-purpose --clear`
- **THEN** Tala updates only that identity's purpose metadata and returns its current purpose and update time
- **AND** the command requires the selected private identity

#### Scenario: Purpose appears in identity and peer listings
- **WHEN** an agent runs `tala whoami` or `tala agents`, including a project-filtered listing
- **THEN** each returned agent record includes `purpose` and `purpose_updated_at`, using null when no purpose is set

#### Scenario: Human purpose is safe and bounded
- **WHEN** an agent runs `whoami --format=text` or `agents --format=text`
- **THEN** Tala prints a readable identity or listing and displays a purpose of at most 80 Unicode scalar values
- **AND** any control characters in stored purpose text are escaped

#### Scenario: JSON purpose is complete
- **WHEN** an agent requests JSON from `whoami` or `agents`
- **THEN** Tala returns the full purpose without the human-output truncation

#### Scenario: Rename remains display-name-only
- **WHEN** an agent runs `tala rename --name=<new-name>`
- **THEN** the display name changes without changing purpose

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
- **WHEN** an agent renders an amended message or its predecessor with `--format=text`
- **THEN** output states which message supersedes which

### Requirement: Rename command
The CLI SHALL expose `tala rename --name=<new-name>` to rename the selected identity. It SHALL use the selected private handle, return the updated public agent record with the previous name and a `changed` flag, and SHALL provide a human-readable confirmation under `--format=text`. The command SHALL reject a blank name. It SHALL not issue or change a private handle.

#### Scenario: Rename command output
- **WHEN** an agent runs `tala rename --name=<new-name>` with a selected identity
- **THEN** the JSON response reports the same public agent ID, the new name, previous name, and `changed:true`
- **AND** `--format=text` confirms the rename without exposing a private handle

#### Scenario: Rename requires identity
- **WHEN** an agent runs `tala rename` without selecting an identity
- **THEN** the command returns the standard identity-required error

### Requirement: Human recipient presence wording
Human-readable output for direct sends and direct replies SHALL describe recipient presence as a point-in-time observation. When the observation says the recipient was not listening, output SHALL state that the message remains queued.

#### Scenario: Direct message recipient is not listening
- **WHEN** a direct send or direct reply is rendered with `--format=text` and `recipient_presence.listening` is false
- **THEN** output says the recipient was not listening and the message remains queued

#### Scenario: Blocking send includes recipient observation
- **WHEN** a direct `send --wait` result is rendered with `--format=text`
- **THEN** output includes the observation from the stored send result, including when the correlated reply wait times out

### Requirement: Status output is consistent across formats
Status SHALL report selector, identity, project, and board high-water context consistently in JSON and human-readable output.

#### Scenario: Human status shows context and availability
- **WHEN** status runs with `--format=text`
- **THEN** it labels the selector source, identity ID/name, registered project root, and board latest ID when available
- **AND** it clearly explains when identity context or the board high-water mark is unavailable

#### Scenario: Board high-water mark is read-only
- **WHEN** status reports `board_latest_id`
- **THEN** the value is the latest message ID in the registered project's board at inspection time
- **AND** reading it does not acknowledge a board message or persist a per-agent cursor

### Requirement: Render thread summaries in both formats
Thread-list JSON SHALL return rows with public thread summary fields and paging metadata. Human output SHALL render the same information as a compact escaped table.

#### Scenario: JSON thread-list fields
- **WHEN** an agent requests a thread list in JSON mode
- **THEN** each row includes thread ID, conversational participants, separate handoff-only recipients, last message ID and time, message count, unread and fresh/stale pending counts, kind, relevant project, and a bounded excerpt

#### Scenario: Human thread-list parity
- **WHEN** an agent runs `tala history --format=text` without a thread ID
- **THEN** Tala prints the same thread rows and summary values in a compact escaped table

### Requirement: Status reports selected identity context
`tala status` SHALL identify the selector source and report the selected identity and registered project context when available, without exposing private selector data.

#### Scenario: Status reports an explicit selector
- **WHEN** status runs with `--agent` or `--agent-file`
- **THEN** its JSON data identifies the selector source and, when resolvable, includes the public agent ID, display name, and registered project root
- **AND** it does not include the handle, selector contents, or selector-file path

#### Scenario: Status reports an environment selector
- **WHEN** status runs with `TALA_AGENT` or `TALA_AGENT_FILE` and no explicit selector
- **THEN** its JSON data identifies the environment selector source and reports the same public identity and project fields when available

#### Scenario: Status reports no or invalid identity
- **WHEN** status runs without a selector or with an invalid/conflicting selector
- **THEN** it reports an explicit `none` or `invalid` selection state while still returning daemon diagnostics
- **AND** it does not expose the raw selector value or selector-file path

#### Scenario: Context is available independently of daemon state
- **WHEN** status has a valid selector and the local database is readable while the daemon is stopped, stale, or incompatible
- **THEN** it reports the persisted identity, registered project root, and board latest ID from the local snapshot
- **AND** the daemon's stopped/stale/incompatible state remains separately visible in daemon diagnostics

#### Scenario: Status in remote mode
- **WHEN** status runs with a valid selector in a home connected to a remote daemon
- **THEN** it reports the public agent ID with `unavailable_reason` of `remote_daemon`, because identity rows live in the host's database
- **AND** it does not read or create a local database
