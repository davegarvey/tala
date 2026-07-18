## MODIFIED Requirements

### Requirement: watch/listen/stream/whatsup help text SHALL cross-reference each other
The system SHALL add a "See Also" section in the help text for `tala wait`, `tala listen`, `tala stream`, `tala whatsup`, `tala recap`, and `tala agents` that explains when to use each command.

#### Scenario: tala wait --help shows related commands
- **WHEN** user runs `tala wait --help`
- **THEN** the help text SHALL include: "See also: tala stream (real-time SSE), tala listen (all sessions), tala whatsup (non-blocking), tala recap (transcript)"

#### Scenario: tala listen --help shows related commands
- **WHEN** user runs `tala listen --help`
- **THEN** the help text SHALL include: "See also: tala wait (blocking poll), tala stream (single session SSE), tala whatsup (non-blocking)"

#### Scenario: tala stream --help shows related commands
- **WHEN** user runs `tala stream --help`
- **THEN** the help text SHALL include: "See also: tala listen (all sessions), tala wait (blocking poll), tala whatsup (non-blocking)"

### Requirement: tala wait --new SHALL be renamed to --new-session
The `--new` flag on `tala wait` SHALL be renamed to `--new-session` to make it unambiguous that it waits for a new session, not new messages.

#### Scenario: --new-session replaces --new
- **WHEN** user runs `tala wait --new-session`
- **THEN** it SHALL behave identically to the previous `tala wait --new`
- **AND** `--new` SHALL be kept as a hidden alias for backward compatibility

#### Scenario: --help shows --new-session
- **WHEN** user runs `tala wait --help`
- **THEN** the help text SHALL show `--new-session` as the primary flag name
- **AND** SHALL NOT show `--new` in the help output (hidden alias)

### Requirement: tala listen default SHALL use checkpoint cursor instead of 0
When `tala listen` is called without `--since`, it SHALL use the last-seen cursor from the disk checkpoint (`.tala/cursor`) instead of defaulting to 0. This prevents replaying full session history on every connect. Using `--since 0` SHALL still show full history.

#### Scenario: tala listen without --since shows only new messages
- **GIVEN** the global cursor in `.tala/cursor` is at ID `42`
- **WHEN** user runs `tala listen` (no `--since`)
- **THEN** the system SHALL use `since=42` instead of `since=0`
- **AND** only show messages with ID > 42

#### Scenario: tala listen --since 0 shows full history
- **WHEN** user runs `tala listen --since 0`
- **THEN** the system SHALL show all messages (full history replay, preserving existing behavior)

#### Scenario: cursor is updated during listen session
- **WHEN** `tala listen` receives new messages
- **THEN** the cursor SHALL be updated to the latest message ID (same behavior as `tala whatsup`)

### Requirement: tala agents SHALL include a hint about cross-project discovery
The help text and output of `tala agents` SHALL mention that `tala discover` can find agents in other projects.

#### Scenario: tala agents --help mentions discover
- **WHEN** user runs `tala agents --help`
- **THEN** the help text SHALL include: "See also: tala discover (cross-project)"

#### Scenario: tala agents empty output mentions discover
- **WHEN** user runs `tala agents` and no agents are found
- **THEN** the output SHALL include a hint: "Try `tala discover` to find agents in other projects."
