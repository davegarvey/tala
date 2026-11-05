## ADDED Requirements

### Requirement: Private selector file selection
The CLI SHALL accept a protected selector file through global `--agent-file=<path>` or `TALA_AGENT_FILE`. The file SHALL contain exactly one `th_` handle, optionally followed by one LF or CRLF. Tala SHALL verify that selector files are regular files owned by the current user and unreadable by others. It SHALL reject symlinks and reparse points and fail closed when it cannot verify file security.

#### Scenario: Select identity from a protected file
- **WHEN** a user runs an identity-bound command with `--agent-file=<path>` or sets only `TALA_AGENT_FILE`
- **THEN** Tala reads the private handle from the file and performs the command as that identity

#### Scenario: Conflicting selectors
- **WHEN** both explicit CLI selectors are supplied, or both selector environment variables are set without an explicit CLI selector
- **THEN** Tala fails with an identity or usage error instead of choosing one silently

### Requirement: Save selector during registration
`register --save-handle-to=<path>` SHALL create a new protected selector file without replacing an existing path or changing its parent directory's permissions. The path SHALL be valid UTF-8 so Tala can return it without loss; an invalid path SHALL fail before registration. On success Tala SHALL return the file path and selection instruction without returning the private handle.

#### Scenario: Save selector without exposing handle
- **WHEN** registration runs with `--save-handle-to=<path>` and file creation succeeds
- **THEN** Tala creates a protected, create-only selector file and returns its path without returning the private handle

#### Scenario: Non-UTF-8 selector path rejected before registration
- **WHEN** `register --save-handle-to` receives a path that cannot be represented as UTF-8
- **THEN** Tala returns a usage error before creating an agent

### Requirement: External selector save failure recovery
If registration succeeds but saving the requested external selector file fails, Tala SHALL return the usable private handle once and report why selector-file saving failed.

#### Scenario: Selector save failure remains recoverable
- **WHEN** registration succeeds but `--save-handle-to` cannot create the requested file
- **THEN** Tala returns the usable private handle once and reports why selector-file saving failed
