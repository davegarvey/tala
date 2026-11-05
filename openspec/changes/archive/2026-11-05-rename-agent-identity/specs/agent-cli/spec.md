## MODIFIED Requirements

### Requirement: Complete clean command surface
The system SHALL expose init, register, rename, unregister, whoami, agents, list-projects, close-project, reopen-project, send, inbox, reply, post, board, history, search, pending, resolve, handoff, status and stop, and SHALL return the attention summary when run with no subcommand. Retired session commands and aliases SHALL be absent.

#### Scenario: Clean help
- **WHEN** an agent runs --help
- **THEN** the new surface is listed and use/session/check/listen are absent
- **AND** rename is listed

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

## ADDED Requirements

### Requirement: Rename command
The CLI SHALL expose `tala rename --name=<new-name>` to rename the selected identity. It SHALL use the selected private handle, return the updated public agent record with the previous name and a `changed` flag, and SHALL provide a human-readable confirmation under `--human`. The command SHALL reject a blank name. It SHALL not issue or change a private handle.

#### Scenario: Rename command output
- **WHEN** an agent runs `tala rename --name=<new-name>` with a selected identity
- **THEN** the JSON response reports the same public agent ID, the new name, previous name, and `changed:true`
- **AND** `--human` confirms the rename without exposing a private handle

#### Scenario: Rename requires identity
- **WHEN** an agent runs `tala rename` without selecting an identity
- **THEN** the command returns the standard identity-required error
