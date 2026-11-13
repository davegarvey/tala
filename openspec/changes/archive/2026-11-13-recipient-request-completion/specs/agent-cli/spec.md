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
