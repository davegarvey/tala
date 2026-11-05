# Spec Delta

## MODIFIED Requirements

### Requirement: Complete clean command surface
The system SHALL expose init, register, update-purpose, rename, unregister, whoami, agents, list-projects, close-project, reopen-project, send, inbox, wait, reply, post, board, history, search, pending, resolve, handoff, guide, status and stop, and SHALL return the attention summary when run with no subcommand. Retired session commands and aliases SHALL be absent.

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

## ADDED Requirements

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
