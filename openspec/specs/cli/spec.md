## Purpose

CLI UX and feedback in tala: waiting commands give immediate confirmation, help text is cross-referenced and discoverable, `use` without args lists sessions, and cursors stay fresh after sends — so agents and humans get clear, timely feedback.

## Requirements

### Requirement: Wait --new-session shows initial feedback

When `tala wait --new-session` begins waiting, the system SHALL print an immediate message to stderr confirming it is waiting and showing the timeout value.

#### Scenario: Wait --new with feedback
- **WHEN** user runs `tala wait --new-session --timeout 30`
- **THEN** the system SHALL print `Waiting for a new session (timeout: 30s)...` to stderr before initiating the long poll

### Requirement: Listen help text mentions --since for skipping history

The `tala listen` help text SHALL document the `--since` flag as a way to skip historical message replay. A dedicated `--new-only` flag is deferred to a future loop as it requires server-side changes (the `since` parameter controls both history replay AND new message filtering in the SSE endpoint, so a client-only flag cannot reliably skip history without also suppressing new messages).

#### Scenario: Listen help shows --since
- **WHEN** user runs `tala listen --help`
- **THEN** the help output SHALL mention `--since` as a way to skip historical message replay

### Requirement: Use without args lists sessions

When `tala use` is run without arguments and no active session is set, the system SHALL list available sessions instead of just saying "no active session".

#### Scenario: Use with no active session
- **WHEN** no active session is set
- **AND** user runs `tala use`
- **THEN** the system SHALL display a list of available sessions with their IDs, names, and message counts

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

### Requirement: Wait --new-session mentioned in top-level help

The `tala wait` command doc comment (shown in `tala --help`) SHALL mention `--new-session` as a usage option.

#### Scenario: Wait doc comment includes --new-session
- **WHEN** user runs `tala --help`
- **THEN** the one-line description for `wait` SHALL mention `--new-session`

### Requirement: Cursor updated on send

When `tala send` successfully sends a message, the system SHALL update the local cursor file to include the sent message ID.

#### Scenario: Send updates cursor
- **WHEN** user runs `tala send "hello"`
- **THEN** the cursor file SHALL be updated with the sent message ID
- **AND** subsequent `tala list` SHALL NOT count the sent message as unread

### Requirement: Definitive command set — send, history, check

The message workflow SHALL be built from three commands. `tala send` SHALL send a message to a session, resolved from a positional session ref or `--session`/`-s`, falling back to the active session, with the auto-create and auto-target behaviors defined in message-sending. `--wait`/`-w` SHALL block for a reply, bounded by `--timeout` (in seconds). Message content SHALL come from exactly one of the positional message, `--message-file <path>`, `--stdin`, or piped stdin (see stdin-sending); a send with no content and no active session SHALL fail. `tala history` SHALL show the conversation transcript of a session (positional or `--session`, active session by default) and SHALL support `--since`, `--from`, `--limit`, and `--json`. `tala check` SHALL fetch new messages since each session's stored cursor, display them grouped by session, update the cursors, and return immediately (non-blocking, no SSE), with `--json` output.

#### Scenario: Send message to active session
- **WHEN** active session exists and user runs `tala send "hello"`
- **THEN** the message is delivered and confirmation "✓ Sent message <n> to session <id>" is printed

#### Scenario: Empty send fails
- **WHEN** user runs `tala send` with no message, no piped input, and no active session
- **THEN** the command SHALL fail with "Nothing to send. Use `tala session create` to create a session without a message."

#### Scenario: Send with wait
- **WHEN** user runs `tala send --wait "question?"`
- **THEN** the message is sent and the command blocks until a reply arrives

#### Scenario: View history of active session
- **WHEN** user runs `tala history`
- **THEN** the full transcript of the active session is displayed

#### Scenario: View history of specific session
- **WHEN** user runs `tala history sess_abc123`
- **THEN** the full transcript of that session is displayed

#### Scenario: Check returns new messages
- **WHEN** user runs `tala check` and new messages exist since the stored cursors
- **THEN** the messages are displayed grouped by session

#### Scenario: Check with no new messages
- **WHEN** user runs `tala check` and no messages exist since the stored cursors
- **THEN** the system SHALL print "No new messages since last check"

#### Scenario: Check updates the cursors
- **WHEN** user runs `tala check` and messages were displayed
- **THEN** the per-session cursors SHALL be updated to the highest message IDs seen

### Requirement: Deprecated commands and flags removed

The system SHALL NOT include the legacy `start`, `recap`, `whatsup`, `observe`, `follow`, or `watch` commands, and SHALL NOT ship deprecated aliases that only warn. Session creation without a message is done with `tala session create`; transcripts with `tala history`; new-message checks with `tala check`; all-session observation with `tala listen`; single-session streaming with `tala stream`. `tala send` SHALL NOT accept a `--file` flag (only `--message-file`). `tala wait` SHALL use `--new-session` and SHALL NOT accept a `--new` alias. `tala check` SHALL NOT accept `--cursor` or `--new` flag aliases.

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

### Requirement: `tala wait` lists multiple open sessions without a target

When `tala wait` runs without a session argument and no active session is set, and 2+ open sessions exist, the system SHALL NOT wait: it SHALL print "Multiple open sessions. Use `tala use <id>` to select one:" followed by each session with its id, name, and message count, and exit. In `--json` mode the system SHALL output a single document: `{"sessions": [...], "error": "Use 'tala use <id>' to select a session"}`.

#### Scenario: Multiple open sessions listed with guidance
- **WHEN** user runs `tala wait` with no active session and 2+ open sessions
- **THEN** the system SHALL display "Multiple open sessions. Use `tala use <id>` to select one:"
- **AND** SHALL list each session with its id, name, and message count
- **AND** SHALL NOT wait for messages

#### Scenario: Multiple open sessions with --json
- **WHEN** user runs `tala wait --json` with no active session and 2+ open sessions
- **THEN** the output SHALL be `{"sessions": [{"id": "...", "name": "...", "message_count": N}], "error": "Use 'tala use <id>' to select a session"}`

### Requirement: `tala use` accepts session names

`tala use <ref>` SHALL accept a session name in addition to a session id. Name lookup SHALL run first, across active (non-closed) sessions only; an exact single match sets the active session, multiple matches SHALL error. When no name matches, id matching (exact, then unique prefix) SHALL apply. A ref matching a closed session SHALL error with guidance to run `tala session reopen`. A ref matching nothing SHALL error.

#### Scenario: Use by name
- **WHEN** user runs `tala use beta-watch` and one active session is named "beta-watch"
- **THEN** the active session SHALL be set to that session and a confirmation SHALL be printed

#### Scenario: Use by ambiguous name
- **WHEN** two active sessions have the name "beta-watch" and user runs `tala use beta-watch`
- **THEN** the command SHALL error with "Multiple sessions named 'beta-watch'. Use session ID instead."

#### Scenario: Use by ID still works
- **WHEN** user runs `tala use sess_abc123`
- **THEN** the active session SHALL be set to `sess_abc123`

#### Scenario: Use on a closed session suggests reopen
- **WHEN** user runs `tala use sess_abc` and `sess_abc` exists but is closed
- **THEN** the command SHALL error with "Session 'sess_abc' is closed. Use `tala session reopen sess_abc` to open it, then `tala use sess_abc` to make it active"

### Requirement: `tala close` uses the active session and supports --quiet

`tala close` without a session argument SHALL close the currently active session and print "Session <id>: closed" as confirmation. `--quiet`/`-q` SHALL suppress the human-readable confirmation; `--json` output SHALL NOT be suppressed by `--quiet`. Closing the active session SHALL clear the active-session marker.

#### Scenario: Close without arg closes active session
- **GIVEN** the active session is `sess_abc`
- **WHEN** user runs `tala close`
- **THEN** session `sess_abc` SHALL be closed and "Session sess_abc: closed" SHALL be printed

#### Scenario: Close with --quiet suppresses confirmation
- **WHEN** user runs `tala close sess_abc --quiet`
- **THEN** the session SHALL be closed and no human-readable confirmation SHALL appear on stdout

#### Scenario: Close with --quiet and --json
- **WHEN** user runs `tala close sess_abc --quiet --json`
- **THEN** stdout SHALL contain the JSON response (quiet does not suppress JSON)

### Requirement: Session and use help cross-references

`tala session --help` SHALL annotate the `list` and `close` subcommands with "(alias: `tala list`)" and "(alias: `tala close`)". `tala use --help` SHALL mention `tala session (show, rename, reopen)` for advanced session management.

#### Scenario: Session help shows shortcut hints
- **WHEN** user runs `tala session --help`
- **THEN** the help text for `list` SHALL include "(alias: `tala list`)"
- **AND** the help text for `close` SHALL include "(alias: `tala close`)"

#### Scenario: Use help mentions session subcommand
- **WHEN** user runs `tala use --help`
- **THEN** the help text SHALL include "See also: tala session (show, rename, reopen)"

### Requirement: `tala init` accepts a positional name

`tala init <name>` SHALL write `.tala/config.json` containing `{"name": "<name>"}`. Without a name argument, the current directory's name SHALL be used as the agent name.

#### Scenario: Init with positional name
- **WHEN** user runs `tala init my-project`
- **THEN** `.tala/config.json` SHALL contain `{"name": "my-project"}`

#### Scenario: Init with no name falls back to directory name
- **WHEN** user runs `tala init` from directory `/projects/my-project`
- **THEN** `.tala/config.json` SHALL contain `{"name": "my-project"}`

### Requirement: `tala list` shows session names

The default output of `tala list` SHALL include each session's name (or `-` when unnamed) in a column: `<id>  <name or ->  <status>  <n> msgs`. The name column SHALL be padded to the longest name so columns stay aligned. `--json` output SHALL be unchanged.

#### Scenario: List with named sessions
- **WHEN** user runs `tala list` and a session has name "alpha-task"
- **THEN** the output SHALL contain "alpha-task" in that session's line

#### Scenario: List with unnamed sessions
- **WHEN** user runs `tala list` and a session has no name
- **THEN** the output SHALL show `-` in the name column

#### Scenario: List with mixed name lengths
- **WHEN** sessions have names "a" and "very-long-name"
- **THEN** columns SHALL remain aligned (space-padded to the longest name)

### Requirement: `tala agents` hints at cross-project discovery

`tala agents --help` SHALL reference `tala discover` for finding agents in other projects. When no agents are found, the output SHALL suggest `tala discover`.

#### Scenario: Agents help mentions discover
- **WHEN** user runs `tala agents --help`
- **THEN** the help text SHALL include "See also: tala discover (cross-project agent discovery)"

#### Scenario: Empty agents output mentions discover
- **WHEN** user runs `tala agents` and no agents are found
- **THEN** the output SHALL include a hint: "Try `tala discover` to find agents in other projects."
