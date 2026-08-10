## Purpose

Message sending UX in tala: auto-created sessions announce themselves on stdout, JSON output stays clean, and starting a session with a message stores it exactly once.

## ADDED Requirements

### Requirement: Auto-create notification to stdout

When `tala send` runs without an active session and auto-creates a new session, it SHALL print `→ Created session <id>` to stdout (not stderr). JSON output SHALL NOT include this message (the session_id is in the response body).

#### Scenario: Non-JSON auto-create visible
- **WHEN** user runs `tala send "hello"` with no active session and no `--json`
- **THEN** stdout SHALL contain "→ Created session <id>"
- **THEN** the confirmation "✓ Sent message" SHALL also appear on stdout

#### Scenario: JSON auto-create not interleaved
- **WHEN** user runs `tala send "hello"` with `--json` and no active session
- **THEN** stdout SHALL contain only the JSON response
- **THEN** the "→ Created session" message SHALL NOT appear on stdout

### Requirement: `tala start` no duplicate message

`tala start "message"` SHALL store the initial message as part of session creation (via `CreateSessionRequest.message`). It SHALL NOT send the message again via a separate POST.

#### Scenario: Start with message has exactly one
- **WHEN** user runs `tala start "hello"`
- **THEN** `tala recap` SHALL show exactly one message with content "hello"
- **WHEN** a second agent runs `tala wait --new` before the start
- **THEN** they SHALL receive exactly one message notification
