## Purpose

Structured message content in tala: messages carry an ordered list of typed parts (text, file, data) instead of a single markdown string, so agents can exchange diffs, file references, and structured data unambiguously.

## Requirements

### Requirement: Message parts model

Every `Message` SHALL carry its content as an ordered list of `parts` rather than a single string. A part SHALL be one of three kinds:

- `text`: a markdown string, the primary human-readable content.
- `file`: a file path reference, with an optional short label.
- `data`: a structured JSON value, with an optional short label.

A message SHALL contain at least one part, and every part SHALL be non-empty. All parts of a message SHALL share the message's intent, reply correlation, and waiting metadata.

`tala send` SHALL accept parts via `--part text:<value>`, `--part file:<path>`, and `--part data:<json>`, repeatable, in order. A plain positional content argument SHALL be equivalent to a single text part. `--part` values SHALL count as content for all send purposes: `tala send` with only `--part` flags SHALL NOT fall back to piped stdin, SHALL pass the "nothing to send" gate, and SHALL auto-create a session when no active session exists, exactly as a positional send would. Combining `--part` with a positional content argument, `--message-file`, or `--stdin` SHALL be rejected with a usage error.

#### Scenario: Parts-only send creates a session
- **WHEN** a user runs `tala send --part text:"review the change" --part file:src/api.rs` with no active session
- **THEN** a session SHALL be created and the stored message SHALL have two parts in order: a text part with content "review the change" and a file part referencing `src/api.rs`

#### Scenario: Positional content is a text part
- **WHEN** a user runs `tala send "hello"`
- **THEN** the stored message SHALL have exactly one part, a text part with content "hello"

#### Scenario: Mixing --part with legacy content sources rejected
- **WHEN** a user runs `tala send --part text:"hi" --stdin` or `tala send "hi" --part text:"there"`
- **THEN** the command SHALL fail with a usage error

#### Scenario: Empty parts rejected
- **WHEN** a send is attempted with no parts and no content, or with an empty text part (`--part text:`)
- **THEN** the command SHALL fail with a usage error (exit code 2)

### Requirement: Legacy content compatibility

The daemon SHALL accept a send payload carrying a legacy `content` string in place of `parts` — provided the payload carries a valid `idempotency_key` (see send-idempotency) — storing it as a single text part. Persisted messages stored with a legacy `content` field SHALL load as a single text part. This conversion SHALL apply to every message surface without user-visible difference from an equivalent parts-form message.

Message serialization (wire responses and persisted state) SHALL include a legacy `content` string — the message's text parts newline-joined — alongside the canonical `parts` array, so older clients that only understand `content` continue to parse messages.

#### Scenario: Legacy client send
- **WHEN** a client sends a message with a `content` string, no `parts`, and a valid `idempotency_key`
- **THEN** the daemon SHALL store it as a message with one text part equal to that string

#### Scenario: Responses carry both parts and legacy content
- **WHEN** a message contains text parts "a" and "b"
- **THEN** its JSON SHALL include a `parts` array with both parts and a `content` field equal to "a\nb"

#### Scenario: Legacy persisted messages load
- **WHEN** the daemon starts with a persisted `messages.json` containing messages in the legacy string format
- **THEN** each such message SHALL be available with a single text part, and `history` SHALL render it identically to before

### Requirement: Parts rendering

Every message surface (`history`, `wait`, `stream`, `listen`, `check`) SHALL render text parts as the message content, exactly as a plain string message today. File parts SHALL render as an annotation naming the path (e.g. `[file: src/api.rs]`), and data parts as an annotation with the JSON value (e.g. `[data: {"status":"ok"}]`), shown after the text parts in part order. In `--json` output, the full typed `parts` array SHALL be present on every message. Message match filters (e.g. `listen --match`) SHALL match against the message's text parts only; a message with no text part SHALL NOT match.

#### Scenario: History shows file annotation
- **WHEN** a message contains a text part "fix applied" and a file part `src/api.rs`
- **THEN** `tala history` SHALL render the text "fix applied" and the annotation `[file: src/api.rs]`

#### Scenario: JSON output exposes typed parts
- **WHEN** a user runs `tala history --json` on a session containing a message with text and data parts
- **THEN** the message JSON SHALL include a `parts` array whose elements carry their kind and payload
