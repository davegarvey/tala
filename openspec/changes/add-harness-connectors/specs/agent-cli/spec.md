# Spec Delta

## MODIFIED Requirements

### Requirement: Complete clean command surface
The system SHALL expose init, register, update-purpose, rename, unregister, whoami, agents, project (with list, close and reopen), send, inbox, wait, reply, accept, decline, complete, post, board, history, search, pending, resolve, handoff, guide, status, stop, network (with open, extend, close, rotate-token, status, connect and disconnect), connector (with list, install, remove, check and hook), and SHALL return the attention summary when run with no subcommand. Retired session commands and aliases, including `list-projects`, `close-project`, `reopen-project`, `enable-network`, `disable-network`, `connect-remote` and `disconnect-remote`, SHALL be absent.

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
- **AND** connector commands appear only under `tala connector`

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

### Requirement: Agent integration
Init SHALL generate versioned generic agent instructions and OpenCode integrations when present, preserve customized documents unless refresh is explicit, and support check and dry-run. Instructions SHALL require distinct per-session registration, keeping the private handle out of project files and shared environment, checking `tala` at work boundaries, CLI-only messaging and retention of user authorization boundaries. Init SHALL not install harness hooks or change harness configuration; only explicit `tala connector install` and `tala connector remove` commands SHALL change harness configuration.

#### Scenario: Safe integration
- **WHEN** init encounters customized instructions
- **THEN** files remain intact until explicit refresh

#### Scenario: Instructions alone suffice
- **WHEN** an agent follows only the generated instructions
- **THEN** it can register, find pending work with `tala`, reply and wait without any harness integration

#### Scenario: Init leaves harness configuration alone
- **WHEN** a user runs `tala init` in a project that has `.claude/`, `.codex/` or `.opencode/` configuration
- **THEN** no harness hook or settings file is created or changed
