## Purpose

Defines agent cli for reliable local collaboration between distinct coding agents in shared and separate projects.

## Requirements

### Requirement: Complete clean command surface
The system SHALL expose init, register, update-purpose, rename, unregister, whoami, agents, list-projects, close-project, reopen-project, send, inbox, wait, reply, accept, decline, post, board, history, search, pending, resolve, handoff, guide, status and stop, and SHALL return the attention summary when run with no subcommand. Retired session commands and aliases SHALL be absent.

#### Scenario: Accept or decline through the CLI
- **WHEN** the recipient runs `tala accept <id>` or `tala decline <id>`
- **THEN** Tala returns the recorded request decision and optional note or reason

#### Scenario: Clean help
- **WHEN** an agent runs --help
- **THEN** the new surface is listed and use/session/check/listen are absent
- **AND** rename is listed
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
- **WHEN** an agent runs `tala guide` without `--human`
- **THEN** Tala returns the complete bundled guide as the `content` string in the standard JSON success envelope and reports the available topics
- **AND** the command does not require a selected identity or running daemon

#### Scenario: Read a guide topic
- **WHEN** an agent runs `tala guide listening`, `requests`, `identity`, `worktrees` or `errors`
- **THEN** Tala returns the requested topic content and its topic slug in the standard JSON success envelope

#### Scenario: Human-readable guide
- **WHEN** an agent runs `tala guide [topic] --human`
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
- **WHEN** an agent runs `whoami --human` or `agents --human`
- **THEN** Tala prints a readable identity or listing and displays a purpose of at most 80 Unicode scalar values
- **AND** any control characters in stored purpose text are escaped

#### Scenario: JSON purpose is complete
- **WHEN** an agent requests JSON from `whoami` or `agents`
- **THEN** Tala returns the full purpose without the human-output truncation

#### Scenario: Rename remains display-name-only
- **WHEN** an agent runs `tala rename --name=<new-name>`
- **THEN** the display name changes without changing purpose

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

### Requirement: Agent integration
Init SHALL generate versioned generic agent instructions and OpenCode integrations when present, preserve customized documents unless refresh is explicit, and support check and dry-run. Instructions SHALL require distinct per-session registration, keeping the private handle out of project files and shared environment, checking `tala` at work boundaries, CLI-only messaging and retention of user authorization boundaries. Init SHALL not install harness hooks or change harness configuration.

#### Scenario: Safe integration
- **WHEN** init encounters customized instructions
- **THEN** files remain intact until explicit refresh

#### Scenario: Instructions alone suffice
- **WHEN** an agent follows only the generated instructions
- **THEN** it can register, find pending work with `tala`, reply and wait without any harness integration

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

#### Scenario: Thread outside the visible scope
- **WHEN** an agent accesses a thread it cannot see through its selected identity and project context
- **THEN** the error uses the generic `THREAD_NOT_VISIBLE` code and does not disclose whether the thread exists or who owns it
- **AND** its hint explains safe recovery through the original private identity or a visible participant's handoff, without exposing handles or foreign project metadata
- **AND** its `next` command is `tala guide identity`

#### Scenario: Reply to a superseded request
- **WHEN** an agent runs `tala reply <message-id>` for a request with a successor
- **THEN** Tala returns `REQUEST_SUPERSEDED` with `retry` of `after_fix` and no stored message ID
- **AND** the error names the successor ID and provides one exact command to inspect the thread

### Requirement: Exit codes
The CLI SHALL exit 0 on success, 1 on operational error with nothing stored, 2 on usage error, 3 on wait timeout, 4 when a message was stored but a subsequent wait failed, and 5 when an active wait is ended by an accepted daemon stop. Help output SHALL list these codes. A `DAEMON_STOPPED` result SHALL be terminal to a supervisor and SHALL NOT cause automatic rearming. When `send --wait` times out after storing its request, its result SHALL identify the stored request and provide one exact command to resume waiting for that request without resending it.

#### Scenario: Active wait is stopped
- **WHEN** `tala wait` or `tala inbox --wait` is active when an accepted stop advances the lifecycle generation
- **THEN** it exits 5 with `DAEMON_STOPPED` and does not start or adopt a replacement daemon

#### Scenario: New wait begins during shutdown
- **WHEN** a new wait command starts while lifecycle state marks an instance as stopping
- **THEN** it exits 5 immediately without waiting for the old daemon or starting a replacement

#### Scenario: Inbox receipt survives stop race
- **WHEN** `inbox --wait` consumes and acknowledges a nonempty batch before or during a racing stop
- **THEN** it returns the consumed messages with exit 0
- **AND** a supervisor's next wait invocation exits 5 if shutdown is still active

#### Scenario: Stored then failed
- **WHEN** `send --wait` stores its request and the wait then fails permanently
- **THEN** exit is 4 and the error's `stored` field holds the message ID

#### Scenario: Stored request times out
- **WHEN** `send --wait --timeout=N` stores its request but receives no correlated reply before the deadline
- **THEN** it exits 3, reports that the request is stored and timed out, and provides a `next` command using `inbox --wait --reply-to=<message-id> --timeout=N`
- **AND** the command does not suggest resending the request

### Requirement: Project discovery without identity
The `list-projects` command SHALL allow read-only project discovery without a selected agent identity. Supplying a valid identity SHALL remain optional and preserve the authenticated response behavior.

#### Scenario: List projects before registration
- **WHEN** a user runs `tala list-projects` without `--agent` or `TALA_AGENT`
- **THEN** Tala returns the server-wide project list with the normal filters, pagination, and hidden-project counts

#### Scenario: Authenticated project listing
- **WHEN** a registered agent runs `tala list-projects` with its selected identity
- **THEN** Tala returns the same project listing and the agent's response includes its normal ambient attention summary

### Requirement: Unified background wait
The CLI SHALL expose `tala wait` to monitor unread inbox messages, new messages on the selected project's board, and durable lifecycle or delivery events. Its timeout SHALL default to indefinite; finite `--timeout` SHALL remain available. Inbox and board reads SHALL not acknowledge messages. A new wait without `--board-after` SHALL use the project's atomic latest-message ID as its baseline. A wait SHALL bind to its lifecycle generation and SHALL exit 5 with `DAEMON_STOPPED` if an accepted stop advances that generation. The client SHALL block on a daemon-held wait request rather than periodically polling.

#### Scenario: Long-running wait wakes on a durable event
- **WHEN** a sender has an indefinite wait open and the daemon records acceptance, decline, deadline, or delivery-overdue
- **THEN** the same wait returns promptly with that event

#### Scenario: Wait recovers notifications after disconnect
- **WHEN** the client reconnects or restarts from its event cursor
- **THEN** durable events remain available and the returned cursor advances only through returned events

#### Scenario: Wait for direct messages and correlated replies
- **WHEN** an agent runs `tala wait` and an unread direct message or reply arrives
- **THEN** the command returns the event ID and type without its body and leaves it unread
- **AND** its next commands fetch the inbox and resume with the returned inbox and board cursors

#### Scenario: New wait board baseline
- **WHEN** a new `tala wait` starts without `--board-after` while the board already contains messages
- **THEN** existing board messages do not immediately wake it
- **AND** the returned continuation command carries the baseline cursor

#### Scenario: Daemon lacks the atomic board high-water mark
- **WHEN** `tala wait` receives a board response without `latest_id`
- **THEN** it returns an actionable protocol-compatibility error instead of using a partial page cursor

#### Scenario: Wait stops instead of recovering
- **WHEN** an accepted stop advances the waiter's captured lifecycle generation
- **THEN** the wait exits 5 with `DAEMON_STOPPED` without recovering the daemon

### Requirement: Wait cursors and recovery
The command SHALL accept separate inbox, board and durable event cursors, return event IDs and types without message bodies, and provide exact fetch and restart commands. It SHALL report whether any source has another page. Each cursor SHALL advance only to the last event included in its returned page. Replaying an older cursor MAY repeat events but SHALL NOT hide or acknowledge them.

#### Scenario: Event cursor paginates without skipping
- **WHEN** more durable events exist than fit in one wait response
- **THEN** the next event cursor identifies the last included event and later events remain available

#### Scenario: Supervisor stops after daemon stop
- **WHEN** a wait loop receives exit 5 with `DAEMON_STOPPED`
- **THEN** it exits without issuing another wait command

#### Scenario: Wait for board posts and replies
- **WHEN** an agent waits on its default board or an explicit `--project` board and a new message from another agent appears
- **THEN** the command returns its ID and event type without acknowledging it
- **AND** a board reply also present in the inbox appears once with both event sources

#### Scenario: Resume without skipping an inbox page
- **WHEN** more unread messages exist than fit in one wait result
- **THEN** the result's inbox cursor is the last ID actually returned
- **AND** the exact continuation command returns the remaining unread messages

#### Scenario: Restart after a lost initial wait
- **WHEN** an agent stops a brand-new wait before receiving its first result and starts another wait without a saved board cursor
- **THEN** the new wait establishes a fresh board baseline, while the intervening board history remains available to `tala board`

#### Scenario: Finite wait timeout
- **WHEN** `tala wait --timeout=N` receives no new event before its deadline
- **THEN** it exits 3 with no events and exact continuation commands using the unchanged cursors


### Requirement: Rename command
The CLI SHALL expose `tala rename --name=<new-name>` to rename the selected identity. It SHALL use the selected private handle, return the updated public agent record with the previous name and a `changed` flag, and SHALL provide a human-readable confirmation under `--human`. The command SHALL reject a blank name. It SHALL not issue or change a private handle.

#### Scenario: Rename command output
- **WHEN** an agent runs `tala rename --name=<new-name>` with a selected identity
- **THEN** the JSON response reports the same public agent ID, the new name, previous name, and `changed:true`
- **AND** `--human` confirms the rename without exposing a private handle

#### Scenario: Rename requires identity
- **WHEN** an agent runs `tala rename` without selecting an identity
- **THEN** the command returns the standard identity-required error

### Requirement: Human recipient presence wording
Human-readable output for direct sends and direct replies SHALL describe recipient presence as a point-in-time observation. When the observation says the recipient was not listening, output SHALL state that the message remains queued.

#### Scenario: Direct message recipient is not listening
- **WHEN** a direct send or direct reply is rendered with `--human` and `recipient_presence.listening` is false
- **THEN** output says the recipient was not listening and the message remains queued

#### Scenario: Blocking send includes recipient observation
- **WHEN** a direct `send --wait` result is rendered with `--human`
- **THEN** output includes the observation from the stored send result, including when the correlated reply wait times out

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

### Requirement: Status output is consistent across formats
Status SHALL report selector, identity, project, and board high-water context consistently in JSON and human-readable output.

#### Scenario: Human status shows context and availability
- **WHEN** status runs with `--human`
- **THEN** it labels the selector source, identity ID/name, registered project root, and board latest ID when available
- **AND** it clearly explains when identity context or the board high-water mark is unavailable

#### Scenario: Board high-water mark is read-only
- **WHEN** status reports `board_latest_id`
- **THEN** the value is the latest message ID in the registered project's board at inspection time
- **AND** reading it does not acknowledge a board message or persist a per-agent cursor

### Requirement: List recent threads from history
The CLI SHALL allow `tala history` without a thread ID to list recent threads, while preserving `tala history <thread>` as the full thread reader. The list SHALL support a bounded `--limit` and a separate `--cursor`; the in-thread `--after` cursor SHALL remain exclusive to full thread reads.

#### Scenario: List recent threads
- **WHEN** an agent runs `tala history` with no thread ID
- **THEN** Tala returns recent threads visible to that selected identity with `has_more`, `next_cursor`, and the snapshot high-water message ID

#### Scenario: Continue thread-list paging
- **WHEN** an agent runs `tala history --cursor=<cursor>`
- **THEN** Tala returns the next page from the same listing snapshot

#### Scenario: Preserve full thread reads
- **WHEN** an agent runs `tala history <thread> --after=<message-id>`
- **THEN** Tala returns the existing paginated full thread history behavior

#### Scenario: Keep the cursor modes separate
- **WHEN** a history invocation combines a thread ID with `--cursor` or omits the thread ID while using `--after`
- **THEN** the CLI returns a structured usage error without making an RPC

### Requirement: Render thread summaries in both formats
Thread-list JSON SHALL return rows with public thread summary fields and paging metadata. Human output SHALL render the same information as a compact escaped table.

#### Scenario: JSON thread-list fields
- **WHEN** an agent requests a thread list in JSON mode
- **THEN** each row includes thread ID, conversational participants, separate handoff-only recipients, last message ID and time, message count, unread and fresh/stale pending counts, kind, relevant project, and a bounded excerpt

#### Scenario: Human thread-list parity
- **WHEN** an agent runs `tala history --human` without a thread ID
- **THEN** Tala prints the same thread rows and summary values in a compact escaped table

### Requirement: Request deadline and delivery options
Request send SHALL accept an optional acceptance deadline duration; omission SHALL preserve existing behavior. `tala pending` SHALL expose request acceptance status. A configurable overdue threshold SHALL apply to unread direct messages and delivery notifications SHALL not imply execution failure.

#### Scenario: Acceptance deadline is opt-in
- **WHEN** a request is sent without an acceptance deadline
- **THEN** no acceptance deadline is scheduled
