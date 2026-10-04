## Purpose

Defines agent cli for reliable local collaboration between distinct coding agents in shared and separate projects.

## ADDED Requirements

### Requirement: Complete clean command surface
The system SHALL expose init, register, unregister, whoami, agents, send, inbox, reply, post, board, history, search, pending, resolve, handoff, status and stop. Retired session commands and aliases SHALL be absent.

#### Scenario: Clean help
- **WHEN** an agent runs --help
- **THEN** the new surface is listed and use/session/check/listen are absent

### Requirement: Input and output contract
Send, reply, post and handoff SHALL accept exactly one positional text, message-file, stdin or ordered typed parts input; piped stdin SHALL work implicitly. Global --json SHALL emit one envelope with ok and data or error with code and hint, including usage failures. List output SHALL default to 50 items, cap at 500, and support incremental reads.

#### Scenario: Invalid JSON input
- **WHEN** a JSON invocation has conflicting content sources
- **THEN** exit is 2 and stderr contains a parseable structured error

### Requirement: Agent integration
Init SHALL generate versioned generic agent instructions and OpenCode integrations when present, preserve customized documents unless refresh is explicit, and support check and dry-run. Instructions SHALL require distinct registration, CLI-only messaging and retention of user authorization boundaries.

#### Scenario: Safe integration
- **WHEN** init encounters customized instructions
- **THEN** files remain intact until explicit refresh
