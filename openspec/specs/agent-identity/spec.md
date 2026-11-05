## Purpose

Defines agent identity for reliable local collaboration between distinct coding agents in shared and separate projects.

## Requirements

### Requirement: Independent instances
The system SHALL register distinct agent IDs with name, tool, project checkout and context, even when names or checkouts match. Commands SHALL select identity only through a private handle given with --agent, TALA_AGENT, --agent-file or TALA_AGENT_FILE, never project-local active state.

#### Scenario: Two agents share a checkout
- **WHEN** Claude and Codex register from one checkout
- **THEN** their IDs, handles and inbox state differ and nested directories resolve to the same checkout

### Requirement: Discovery and presence
The system SHALL list registered agents with IDs, context, last_seen, inactivity and listening status. Exact IDs SHALL resolve uniquely; ambiguous names SHALL fail with candidate IDs. Unregister SHALL mark an instance inactive without deleting its history.

#### Scenario: Duplicate names
- **WHEN** two agents have the name codex
- **THEN** addressing codex fails with candidate IDs and an exact ID succeeds

### Requirement: Private identity handle
`register` SHALL return a private handle, distinct from the public agent ID, that is required to act as that agent. Public IDs SHALL be used only for addressing and display. Supplying a public ID to `--agent` or `TALA_AGENT` SHALL fail with `IDENTITY_ERROR` and a hint explaining the difference. Handles SHALL never appear in messages, listings or other agents' output.

#### Scenario: Public ID cannot select identity
- **WHEN** an agent passes another agent's `agt_` ID from `tala agents` to `--agent`
- **THEN** the command fails with `IDENTITY_ERROR` and no message is sent or consumed

#### Scenario: Handle not disclosed
- **WHEN** any agent lists agents, reads history or searches messages
- **THEN** no output contains another agent's handle

### Requirement: Existing identities upgrade explicitly
Identities created before handles existed SHALL remain addressable and keep their history. Their owner SHALL obtain a handle through an explicit `register --reissue=<agt_id>`, a transitional option hidden from help, which SHALL succeed at most once per identity and retire the ID-keyed credential. No handle SHALL be issued implicitly.

#### Scenario: Reissue for an existing identity
- **WHEN** an agent with a protocol 2 identity runs `register --reissue=<its id>`
- **THEN** it receives a handle for the same agent ID and its existing inbox remains intact

#### Scenario: Second reissue refused
- **WHEN** `register --reissue` runs again for an identity that already has a handle
- **THEN** it fails with `IDENTITY_ERROR` and the existing handle remains valid

### Requirement: Registration handle save recovery
When registration is committed but saving its private handle fails, Tala SHALL preserve the credential in private storage and report a direct command to issue a handle for the same agent ID.

#### Scenario: Reissue after a handle save failure
- **WHEN** registration succeeds but the new handle file cannot be saved and the private recovery credential is saved
- **THEN** the error identifies the agent and provides `tala register --reissue=<agent-id>` to recover that identity
- **AND** the reissued handle selects the same agent and preserves its history

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
