## MODIFIED Requirements

### Requirement: CLI command naming — observe renamed to listen, follow renamed to watch
The system SHALL rename the `observe` command to `listen` and the `follow` command to `watch`. The old names SHALL be preserved as hidden aliases for backward compatibility. Using an alias SHALL emit a deprecation warning to stderr. The `/api/observe` and `/api/sessions/:id/events` (used by follow/watch) daemon routes SHALL keep their current paths — `cmd_listen` still calls `/api/observe` and `cmd_watch` still calls `/api/sessions/:id/events`.

#### Scenario: observe becomes listen
- **WHEN** user runs `chit listen --help`
- **THEN** the system SHALL show the help text previously shown by `chit observe --help`

#### Scenario: observe alias still works with warning
- **WHEN** user runs `chit observe`
- **THEN** the system SHALL execute the same behavior as `chit listen`
- **AND** SHALL print a deprecation warning to stderr: `"warning: 'observe' is deprecated, use 'chit listen' instead"`

#### Scenario: follow becomes watch
- **WHEN** user runs `chit watch --help`
- **THEN** the system SHALL show the help text previously shown by `chit follow --help`

#### Scenario: follow and stream aliases still work with warnings
- **WHEN** user runs `chit follow`
- **THEN** the system SHALL execute the same behavior as `chit watch`
- **AND** SHALL print a deprecation warning to stderr

### Requirement: chit wait without session SHALL show helpful guidance
When `chit wait` is called without a session argument and no active session is set, the system SHALL list available sessions or show actionable guidance instead of a raw SESSION_NOT_FOUND error. The existing blocking behavior for 0 open sessions (waiting for a new session to be created) SHALL be preserved for backward compatibility.

#### Scenario: No sessions available (preserves blocking behavior)
- **WHEN** user runs `chit wait` with no active session and no open sessions
- **THEN** the system SHALL block and wait for a new session (current behavior preserved)

#### Scenario: Multiple open sessions
- **WHEN** user runs `chit wait` with no active session and 2+ open sessions
- **THEN** the system SHALL display a message: "Multiple open sessions. Use `chit use <id>` to select one:"
- **AND** SHALL list each session with its ID, name, and message count
- **AND** SHALL NOT wait for messages

#### Scenario: Multiple open sessions with --json
- **WHEN** user runs `chit wait --json` with no active session and 2+ open sessions
- **THEN** the system SHALL output: `{"sessions": [{"id": "...", "name": "...", "message_count": N}], "error": "Use 'chit use <id>' to select a session"}`

### Requirement: Old aliases SHALL have test coverage
The system SHALL include tests verifying alias commands still function and emit deprecation warnings.

#### Scenario: observe alias test
- **WHEN** tests invoke `chit observe`
- **THEN** the behavior SHALL match `chit listen`

#### Scenario: follow alias test
- **WHEN** tests invoke `chit follow`
- **THEN** the behavior SHALL match `chit watch`

### Requirement: README and embedded SKILL.md SHALL be updated
The system SHALL update all documentation references from `observe` to `listen` and `follow` to `watch`.

#### Scenario: README updated
- **WHEN** user views the README
- **THEN** the command table SHALL reference `listen` and `watch` instead of `observe` and `follow`

#### Scenario: Init SKILL.md updated
- **WHEN** user runs `chit init`
- **THEN** the generated SKILL.md SHALL reference `listen` and `watch` instead of `observe` and `follow`
