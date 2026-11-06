# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Request deadline and delivery options
Request send SHALL accept an optional acceptance deadline duration; omission SHALL preserve existing behavior. `tala pending` SHALL expose request acceptance status. A configurable overdue threshold SHALL apply to unread direct messages and delivery notifications SHALL not imply execution failure.

#### Scenario: Acceptance deadline is opt-in
- **WHEN** a request is sent without an acceptance deadline
- **THEN** no acceptance deadline is scheduled
