## Purpose

Defines agent cli for reliable local collaboration between distinct coding agents in shared and separate projects.

## Requirements

### Requirement: Complete clean command surface
The system SHALL expose init, register, unregister, whoami, agents, list-projects, close-project, reopen-project, send, inbox, reply, post, board, history, search, pending, resolve, handoff, status and stop, and SHALL return the attention summary when run with no subcommand. Retired session commands and aliases SHALL be absent.

#### Scenario: Clean help
- **WHEN** an agent runs --help
- **THEN** the new surface is listed and use/session/check/listen are absent

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

### Requirement: Input and output contract
Send, reply, post and handoff SHALL accept exactly one positional text, message-file, stdin or ordered typed parts input; piped stdin SHALL work implicitly. Output SHALL default to one JSON envelope with ok and data or error, including usage failures; `--human` SHALL select text rendering and `--json` SHALL be accepted without effect and hidden from help. List output SHALL default to 50 items, cap at 500, and support incremental reads.

#### Scenario: Invalid JSON input
- **WHEN** an invocation has conflicting content sources
- **THEN** exit is 2 and stderr contains a parseable structured error

#### Scenario: JSON without a flag
- **WHEN** an agent runs `tala inbox` without `--json` or `--human`
- **THEN** stdout is a single parseable JSON envelope

#### Scenario: Human rendering on request
- **WHEN** an agent runs `tala inbox --human`
- **THEN** stdout is the text rendering

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

### Requirement: Exit codes
The CLI SHALL exit 0 on success, 1 on operational error with nothing stored, 2 on usage error, 3 on wait timeout and 4 when a message was stored but a subsequent wait failed. Help output SHALL list these codes. When `send --wait` times out after storing its request, its result SHALL identify the stored request and provide one exact command to resume waiting for that request without resending it.

#### Scenario: Stored then failed
- **WHEN** `send --wait` stores its request and the wait then fails permanently
- **THEN** exit is 4 and the error's `stored` field holds the message ID

#### Scenario: Stored request times out
- **WHEN** `send --wait --timeout=N` stores its request but receives no correlated reply before the deadline
- **THEN** it exits 3, reports that the request is stored and timed out, and provides a `next` command using `inbox --wait --reply-to=<message-id> --timeout=N`
- **AND** the command does not suggest resending the request
