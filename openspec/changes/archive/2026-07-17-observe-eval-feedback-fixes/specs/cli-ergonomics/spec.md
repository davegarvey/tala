## ADDED Requirements

### Requirement: `chit init` accepts positional name argument
`chit init` SHALL accept an optional positional name argument: `chit init <name>`. It SHALL also continue to support the `--name` flag for backward compatibility. The positional and `--name` SHALL be configured as conflicting args in clap — if both are provided, the command SHALL error with a conflict message rather than silently choosing one.

#### Scenario: Init with positional name
- **WHEN** user runs `chit init my-project`
- **THEN** `.chit/config.json` SHALL contain `{"name": "my-project"}`

#### Scenario: Init with --name flag
- **WHEN** user runs `chit init --name my-project`
- **THEN** `.chit/config.json` SHALL contain `{"name": "my-project"}`

#### Scenario: Init with no name falls back to directory name
- **WHEN** user runs `chit init` from directory `/projects/my-project`
- **THEN** `.chit/config.json` SHALL contain `{"name": "my-project"}`

#### Scenario: Both positional and --name errors
- **WHEN** user runs `chit init my-project --name other-name`
- **THEN** the command SHALL error with "The argument '--name' cannot be used with '<name>'"
- **THEN** no config file SHALL be written

### Requirement: `chit list` shows session names in default output
The default (non-JSON) output of `chit list` SHALL include the session name alongside the session ID. Format: `<id>  <name or "-">  <status>  <n> msgs`. The columns SHALL be space-separated and aligned with tab-stops or consistent spacing. The `--json` output SHALL be unchanged.

#### Scenario: List with named sessions
- **WHEN** user runs `chit list` and a session has name "alpha-task"
- **THEN** the output SHALL contain "alpha-task" in the session's line
- **WHEN** user runs `chit list --json`
- **THEN** the output SHALL be valid JSON with the name field

#### Scenario: List with unnamed sessions
- **WHEN** user runs `chit list` and a session has no name
- **THEN** the output SHALL show `-` in the name column

#### Scenario: List with mixed name lengths
- **WHEN** sessions have names "a" and "very-long-name"
- **THEN** columns SHALL remain aligned (space-padded to longest name)

### Requirement: `chit use` accepts session names
`chit use` SHALL accept a session name in addition to a session ID. When given a name, it SHALL look up the session by name across active (non-closed) sessions only. If multiple sessions share the same name, the command SHALL error. If no active session has that name, the command SHALL error.

#### Scenario: Use by name
- **WHEN** user runs `chit start --name beta-watch`
- **AND** user runs `chit use beta-watch`
- **THEN** the active session SHALL be set to the session named "beta-watch"
- **THEN** `chit use` SHALL confirm "Active session set to <id>"

#### Scenario: Use by ambiguous name
- **WHEN** two active sessions have the name "beta-watch"
- **AND** user runs `chit use beta-watch`
- **THEN** the command SHALL error with "Multiple sessions named 'beta-watch'. Use session ID instead."

#### Scenario: Use by non-existent name
- **WHEN** no active session has the name "beta-watch"
- **AND** user runs `chit use beta-watch`
- **THEN** the command SHALL error with "No active session named 'beta-watch'"

#### Scenario: Use by ID still works
- **WHEN** user runs `chit use sess_abc123`
- **THEN** the active session SHALL be set to `sess_abc123`

#### Scenario: Use ignores closed sessions in name lookup
- **WHEN** a closed session exists with name "beta-watch"
- **AND** no active session has that name
- **AND** user runs `chit use beta-watch`
- **THEN** the command SHALL error with "No active session named 'beta-watch'"

#### Scenario: Use by name with special characters
- **WHEN** a session has name "beta's watch" or "beta-watch (v2)"
- **AND** user runs `chit use beta's watch` or `chit use "beta-watch (v2)"`
- **THEN** the active session SHALL be set to the session with that exact name

#### Scenario: Name that matches both ID prefix and name
- **WHEN** a session has name "sess_abc" and a different session has ID "sess_abc123"
- **AND** user runs `chit use sess_abc`
- **THEN** the command SHALL treat the argument as a name first (match "sess_abc" by name)
- **THEN** only if no name match is found, fall back to ID matching
