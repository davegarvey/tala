## MODIFIED Requirements

### Requirement: `chit start` creates session AND sets active
`chit start` SHALL create a new session and print its ID. It SHALL then call `write_active_session()` so the new session becomes the active session for the current project directory.

#### Scenario: Start creates session and sets it active
- **WHEN** user runs `chit start "hello"` from project-alpha
- **THEN** a new session is created with the initial message
- **THEN** the active session for project-alpha SHALL be set to the new session ID
- **WHEN** user runs `chit send "more work"` without `--session`
- **THEN** the message SHALL be sent to that same session

#### Scenario: Start prints session ID
- **WHEN** user runs `chit start`
- **THEN** the session ID is printed to stdout

## ADDED Requirements

### Requirement: `chit start` auto-names session from project config
When `chit start` is called without `--name`, it SHALL read the project name from `.chit/config.json` and use it as the session's name. If no config exists, the session SHALL be created without a name.

#### Scenario: Start with project config provides session name
- **WHEN** `.chit/config.json` contains `{"name": "alpha-project"}`
- **AND** user runs `chit start "hello"` (no `--name`)
- **THEN** the new session SHALL have name "alpha-project"
- **THEN** `chit list` SHALL show "alpha-project" for that session

#### Scenario: Start with --name overrides config
- **WHEN** `.chit/config.json` contains `{"name": "alpha-project"}`
- **AND** user runs `chit start --name beta-task "hello"`
- **THEN** the new session SHALL have name "beta-task"

#### Scenario: Start without config creates nameless session
- **WHEN** `.chit/config.json` does not exist
- **AND** user runs `chit start "hello"` (no `--name`)
- **THEN** the new session SHALL have no name

### Requirement: `chit send` does not auto-create sessions
When `chit send` runs without `--session` and there is no active session, it SHALL NOT auto-create a new session. Instead, it SHALL list active sessions and suggest using `chit start` or `chit use`.

#### Scenario: Send with no sessions
- **WHEN** no sessions exist
- **AND** user runs `chit send "hello"` without `--session`
- **THEN** the command SHALL error with "No active sessions. Start one with `chit start`"
- **THEN** no session SHALL be created

#### Scenario: Send with one active session suggests use
- **WHEN** exactly one active session exists (e.g., `sess_abc`)
- **AND** user runs `chit send "hello"` without `--session`
- **THEN** the command SHALL error with "No active session set. Use `chit use sess_abc` or `chit use <name>` to set one"
- **THEN** no new session SHALL be created

#### Scenario: Send with multiple sessions lists them
- **WHEN** multiple active sessions exist (e.g., `sess_abc` named "alpha", `sess_def` named "beta")
- **AND** user runs `chit send "hello"` without `--session`
- **THEN** the command SHALL error listing the sessions

### Requirement: `chit session rename` clean quoting
The success message for `chit session rename` SHALL display the new name without JSON quoting characters.

#### Scenario: Rename shows bare name
- **WHEN** user runs `chit session rename sess_abc beta-watch`
- **THEN** the output SHALL be "Session sess_abc renamed to 'beta-watch'"
- **THEN** the name SHALL NOT appear as '"beta-watch"' (with JSON quotes)

### Requirement: `chit send` stale session replacement uses project name
When `chit send` detects the active session file points to a closed or deleted session, it SHALL create a replacement session. The replacement session SHALL inherit the project name from `.chit/config.json` as its session name. This is the only remaining auto-create path (distinct from the "no active session" path which errors).

#### Scenario: Stale session replacement gets project name
- **WHEN** `.chit/config.json` contains `{"name": "alpha-project"}`
- **AND** the active session `sess_abc` has been closed
- **AND** user runs `chit send "hello"` (no explicit `--session`)
- **THEN** a new session SHALL be created with name "alpha-project"
- **THEN** the new session SHALL become the active session

#### Scenario: No auto-create when using --session
- **WHEN** user runs `chit send --session sess_valid "hello"`
- **THEN** the message SHALL be sent to `sess_valid`
- **THEN** no new session SHALL be created

### Requirement: `chit send` JSON error output
When `chit send` errors due to no active session and `--json` is set, the error SHALL be a JSON object with an `error` field.

#### Scenario: JSON error on no active session
- **WHEN** no active session is set
- **AND** user runs `chit send --json "hello"`
- **THEN** the output SHALL be `{"error": "No active sessions. Start one with `chit start`"}`
