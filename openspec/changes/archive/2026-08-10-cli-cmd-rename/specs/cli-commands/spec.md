# CLI Commands

Definitive command set for the tala CLI.

## ADDED Requirements

### Requirement: Send a message

The system SHALL provide a `send` command that sends a message to an agent.

- The command SHALL be named `send` (no alias).
- With no active session:
  - If a message is provided: SHALL auto-create a session, send the message, and set the new session as active.
  - If no message is provided: SHALL auto-create an empty session, set it active, and print the session ID.
- With an active session:
  - If a message is provided: SHALL send the message to the active session.
  - If no message is provided: SHALL fail with an error.
- With `--session` / `-s`: SHALL send to the specified session (overrides active).
- With `--wait` / `-w`: SHALL block and wait for a reply after sending.
- With `--wait` and no message: SHALL fail with an error (nothing to send).
- With `--timeout`: SHALL limit wait duration to the specified seconds.
- With `--name` / `-n`: SHALL name the session on auto-create. SHALL fail if used with an explicit `--session`.
- The command SHALL support `--message-file` / `--file`, `--stdin`, positional message, and piped stdin as mutually exclusive input sources.
- The command SHALL NOT accept a deprecated `--file` flag.

#### Scenario: Send message to active session
- **WHEN** active session exists and user runs `tala send "hello"`
- **THEN** message is delivered and session ID is printed

#### Scenario: Auto-create session on send
- **WHEN** no active session exists and user runs `tala send "hello"`
- **THEN** a new session is created, message is delivered, and new session is set as active

#### Scenario: Empty send creates session
- **WHEN** no active session exists and user runs `tala send`
- **THEN** a new empty session is created, set as active, and its ID is printed

#### Scenario: Empty send with active session fails
- **WHEN** active session exists and user runs `tala send`
- **THEN** command fails with error

#### Scenario: Named session
- **WHEN** user runs `tala send --name "my-project" "hello"`
- **THEN** a new session named "my-project" is created and message is delivered

#### Scenario: Send with wait
- **WHEN** user runs `tala send --wait "question?"`
- **THEN** message is sent and command blocks until a reply arrives

#### Scenario: Send with wait but no message fails
- **WHEN** user runs `tala send --wait`
- **THEN** command fails with error "nothing to send"

#### Scenario: Send with --name and --session fails
- **WHEN** user runs `tala send --session "abc" --name "x" "hello"`
- **THEN** command fails with error

---

### Requirement: View conversation history

The system SHALL provide a `history` command that shows a conversation transcript.

- The command SHALL be named `history` (replaces `recap`).
- SHALL accept a session ID (positional or `--session` / `-s`).
- SHALL use the active session if no session is specified.
- SHALL support `--since` to filter messages after an ID.
- SHALL support `--from` to filter by sender.
- SHALL support `--limit` to cap message count.
- SHALL support `--json` / `-j` for JSON output.

#### Scenario: View history of active session
- **WHEN** user runs `tala history`
- **THEN** full transcript of active session is displayed

#### Scenario: View history of specific session
- **WHEN** user runs `tala history sess_abc123`
- **THEN** full transcript of that session is displayed

---

### Requirement: Check for new messages

The system SHALL provide a `check` command that shows new messages since the last check cursor (non-blocking).

- The command SHALL be named `check` (replaces `whatsup`).
- SHALL fetch all sessions and display messages with ID greater than the stored cursor.
- SHALL update the cursor to the highest message ID seen.
- SHALL return immediately (no blocking, no SSE).
- SHALL support `--json` / `-j` for JSON output.

#### Scenario: Check returns new messages
- **WHEN** user runs `tala check` and new messages exist
- **THEN** messages are displayed grouped by session

#### Scenario: Check with no new messages
- **WHEN** user runs `tala check` and no messages since last cursor
- **THEN** informs user "No new messages"

---

### Requirement: Remove deprecated commands

The system SHALL NOT include deprecated command aliases.

- SHALL NOT have `follow`, `watch`, or `observe` commands.
- SHALL NOT have a `--file` flag on `send`.
- SHALL NOT have `--cursor` or `--new` flag aliases.

#### Scenario: Deprecated commands are absent
- **WHEN** user runs `tala follow`
- **THEN** command fails with "not found" error

#### Scenario: Deprecated --file flag is absent
- **WHEN** user runs `tala send --file msg.txt`
- **THEN** command fails with "not found" error

---

### Requirement: Remove start command

The system SHALL NOT include a `start` command.

- Session creation SHALL be handled by `send` (with or without a message).

#### Scenario: Start command is absent
- **WHEN** user runs `tala start`
- **THEN** command fails with "not found" error
