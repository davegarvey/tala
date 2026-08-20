## ADDED Requirements

### Requirement: Public command surface is explicit

The supported top-level Tala command surface SHALL consist of `init`, `use`,
`send`, `wait`, `history`, `listen`, `check`, `list`, `discover`, `close`,
`pending`, `status`, `stop`, and `session`, together with Clap's built-in help
and version options. Repository-local integration documents SHALL be generated
and validated against this surface. Commands removed from this list SHALL NOT
be restored solely to support stale instructions.

#### Scenario: Help exposes the supported public commands

- **WHEN** user runs `tala --help`
- **THEN** the help output SHALL list each supported public command
- **AND** the help output SHALL not list retired commands as supported commands

#### Scenario: Retired command is rejected

- **WHEN** user runs a retired command such as `tala agents` or `tala stream`
- **THEN** the CLI SHALL return a nonzero usage error
- **AND** the CLI SHALL not execute a compatibility alias for that command

## MODIFIED Requirements

### Requirement: Deprecated commands and flags removed

The system SHALL NOT include the legacy `start`, `recap`, `whatsup`, `observe`,
`follow`, `watch`, `stream`, or `agents` commands, and SHALL NOT ship deprecated
aliases that only warn. Session creation without a message is done with `tala
session create`; transcripts with `tala history`; new-message checks with `tala
check`; all-session observation with `tala listen`. `tala send` SHALL NOT accept
a `--file` flag (only `--message-file`). `tala wait` SHALL use `--new-session`
and SHALL NOT accept a `--new` alias. `tala check` SHALL NOT accept `--cursor`
or `--new` flag aliases.

#### Scenario: Start command is absent

- **WHEN** user runs `tala start`
- **THEN** the command fails with a "not found" error

#### Scenario: Deprecated observation commands are absent

- **WHEN** user runs `tala observe` or `tala follow`
- **THEN** the command fails with a "not found" error

#### Scenario: Deprecated --file flag is absent

- **WHEN** user runs `tala send --file msg.txt`
- **THEN** the command fails with an unknown-flag error

#### Scenario: Deprecated --new flag is absent

- **WHEN** user runs `tala wait --new`
- **THEN** the command fails with an unknown-flag error

#### Scenario: Single-session stream command is absent

- **WHEN** user runs `tala stream`
- **THEN** the command fails with a "not found" error
- **AND** the user is directed to `tala wait` or `tala listen` as appropriate

#### Scenario: Local agents listing command is absent

- **WHEN** user runs `tala agents`
- **THEN** the command fails with a "not found" error
- **AND** the user is directed to `tala discover` for cross-project discovery

## REMOVED Requirements

### Requirement: CLI help cross-references wait/stream/listen

The `--help` output for `tala wait`, `tala stream`, and `tala listen` SHALL include brief usage guidance explaining when to use each command.

#### Scenario: Wait help shows cross-references

- **WHEN** user runs `tala wait --help`
- **THEN** the help output SHALL mention `tala stream` for real-time SSE and `tala listen` for observing all sessions

#### Scenario: Stream help shows cross-references

- **WHEN** user runs `tala stream --help`
- **THEN** the help output SHALL mention `tala wait` for blocking poll and `tala listen` for observing all sessions

#### Scenario: Listen help shows cross-references

- **WHEN** user runs `tala listen --help`
- **THEN** the help output SHALL mention `tala stream` for single-session SSE and `tala wait` for blocking poll

**Reason**: The single-session `stream` command is outside the intentionally reduced public surface.

**Migration**: Use `tala wait` for one blocking receive operation or `tala listen` to observe all sessions.

### Requirement: `tala agents` hints at cross-project discovery

`tala agents --help` SHALL reference `tala discover` for finding agents in other projects. When no agents are found, the output SHALL suggest `tala discover`.

#### Scenario: Agents help mentions discover

- **WHEN** user runs `tala agents --help`
- **THEN** the help text SHALL include "See also: tala discover (cross-project agent discovery)"

#### Scenario: Empty agents output mentions discover

- **WHEN** user runs `tala agents` and no agents are found
- **THEN** the output SHALL include a hint: "Try `tala discover` to find agents in other projects."

**Reason**: The local agents aggregation command was removed from the intentionally reduced public surface.

**Migration**: Use `tala discover` for cross-project discovery, and use `tala list` or session history for local session inspection.

### Requirement: CLI help cross-references wait/listen

The `--help` output for `tala wait` and `tala listen` SHALL include brief usage guidance explaining when to use each command.

#### Scenario: Wait help shows cross-references

- **WHEN** user runs `tala wait --help`
- **THEN** the help output SHALL mention `tala listen` for observing all sessions

#### Scenario: Listen help shows cross-references

- **WHEN** user runs `tala listen --help`
- **THEN** the help output SHALL mention `tala wait` for a blocking receive operation
