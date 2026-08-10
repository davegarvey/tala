## Purpose

Message-level intent metadata for tala: senders declare what they expect (reply required, informational, answering, closing), correlate replies to requests, and stamp wait deadlines so recipients see live remaining time — making conversations unambiguous and machine-visible.

## ADDED Requirements

### Requirement: Intent field on messages

Every `Message` SHALL carry an optional `intent` field with one of four values: `req` (a reply is expected), `fyi` (no reply needed; thread continues), `reply` (this message answers a prior request), or `out` (exchange over; no reply expected).

`tala send` SHALL accept `--intent <req|fyi|reply|out>`. When no `--intent` is given, the CLI SHALL resolve the default by this precedence: `--reply-to` present → `reply`; `--wait` present → `req`; otherwise → `fyi`. When `--reply-to` and `--wait` are both present and no `--intent` is given, the intent SHALL be `reply` and the message SHALL be marked `expect_reply` (see Expectation modifier).

Messages received without an `intent` (older clients, session-creation messages) SHALL be treated as `fyi` on the wire.

The intent SHALL be rendered as a visible tag in `history`, `wait`, `stream`, `listen`, `check`, and `pending` output (e.g. `[REQ]`), and SHALL be present in `--json` output for all message surfaces.

#### Scenario: Send with explicit intent
- **WHEN** a user runs `tala send --intent req "help with parse_row"`
- **THEN** the stored message SHALL have `intent: "req"`
- **AND** subsequent `tala history` SHALL render it as `[REQ] help with parse_row`

#### Scenario: Send --wait implies req
- **WHEN** a user runs `tala send --wait "help"`
- **THEN** the stored message SHALL have `intent: "req"`

#### Scenario: Send --reply-to implies reply
- **WHEN** a user runs `tala send --reply-to 5 "fix is in parse_row"`
- **THEN** the stored message SHALL have `intent: "reply"` and `reply_to: 5`

#### Scenario: Send --reply-to with --wait implies reply plus expectation
- **WHEN** a user runs `tala send --reply-to 5 --wait "fix applied — but check my tests"`
- **THEN** the stored message SHALL have `intent: "reply"`, `reply_to: 5`, and `expect_reply: true`

#### Scenario: Plain send defaults to fyi
- **WHEN** a user runs `tala send "status: done"`
- **THEN** the stored message SHALL have `intent: "fyi"`

#### Scenario: Session creation message defaults to fyi
- **WHEN** a session is created with an initial message and no intent is specified
- **THEN** the initial message SHALL have `intent: "fyi"`

#### Scenario: Invalid intent rejected
- **WHEN** a user runs `tala send --intent maybe "hello"`
- **THEN** the command SHALL fail with a usage error listing the four valid values

### Requirement: Reply correlation

Every `Message` SHALL carry an optional `reply_to` field referencing a message id within the same session. `tala send` SHALL accept `--reply-to <message-id>`. The referenced message SHALL be resolved within the target session only.

A request SHALL be considered answered when a later message in the same session either (a) has `reply_to` referencing it, or (b) has `intent: "reply"` without `reply_to` — in which case it answers the oldest unanswered `req` in that session. A sender SHALL be able to close their own open requests by sending `intent: "out"`.

#### Scenario: Send a correlated reply
- **WHEN** message 5 is a `req` and a user runs `tala send --intent reply --reply-to 5 "fix is in parse_row"`
- **THEN** the stored message SHALL have `reply_to: 5`
- **AND** the message SHALL render with its target, e.g. `[REPLY→5]`

#### Scenario: Uncorrelated reply answers oldest open request
- **WHEN** session `sess_ab12` contains unanswered `req` messages 3 and 7
- **AND** a user sends a message with `intent: "reply"` and no `reply_to`
- **THEN** request 3 SHALL be considered answered

#### Scenario: Out closes the sender's open requests
- **WHEN** a user has unanswered `req` messages in a session
- **AND** the user sends `intent: "out"`
- **THEN** all of the sender's unanswered requests in that session SHALL be considered closed

#### Scenario: Reply to a nonexistent message id
- **WHEN** a user runs `tala send --reply-to 999 "hello"` and no message 999 exists in the target session
- **THEN** the command SHALL fail with an error identifying the invalid `--reply-to` id

### Requirement: Expectation modifier

Every `Message` SHALL carry a boolean `expect_reply` field, defaulting to false. `tala send` SHALL accept `--expect-reply` as a modifier valid only with `--intent reply` or `--intent fyi` (or their implied defaults). The modifier SHALL be rejected when combined with `--intent req` or `--intent out`.

A message with `expect_reply: true` SHALL appear in the pending view as an open obligation of its sender.

#### Scenario: Reply with continued expectation
- **WHEN** a user runs `tala send --intent reply --reply-to 5 --expect-reply "fixed — can you re-verify?"`
- **THEN** the stored message SHALL have `expect_reply: true`
- **AND** it SHALL appear in `tala pending` as an open obligation

#### Scenario: Invalid modifier combination rejected
- **WHEN** a user runs `tala send --intent out --expect-reply "bye"`
- **THEN** the command SHALL fail with a usage error

### Requirement: Wait deadline on messages

When `tala send --wait --timeout N` sends a message, the CLI SHALL resolve the effective timeout (the `--timeout` value if given, otherwise the client's configured default) and send it with the message. The daemon SHALL stamp the message with an absolute `waiting_until` timestamp equal to send time plus the resolved timeout. Messages sent without a wait timeout SHALL NOT carry `waiting_until`.

Every message surface (`history`, `check`, `stream`, `wait`, `listen`, `pending`) SHALL render `waiting_until` relative to the current time at render time: "waiting, Ns left" while the deadline is in the future, and "wait expired Xm ago" once past. A message whose deadline has passed SHALL still render its intent as an open obligation — the deadline expresses urgency, not validity.

#### Scenario: Recipient sees remaining time
- **WHEN** a user runs `tala send --wait --timeout 120 "help"` at 21:34:14
- **AND** the recipient runs `tala check` at 21:34:51 (37 seconds later)
- **THEN** the output SHALL show the message with "waiting, 83s left" (remaining computed at read time, not the original 120s)

#### Scenario: Deadline uses the client's effective timeout
- **WHEN** a client whose configured default timeout is 120 runs `tala send --wait "help"` (no `--timeout`)
- **THEN** the stored message SHALL carry `waiting_until` equal to send time plus 120 seconds, matching the client's actual wait window

#### Scenario: Deadline expired but obligation open
- **WHEN** a message has `intent: "req"` and `waiting_until` in the past
- **THEN** message surfaces SHALL render it as "wait expired" rather than "waiting"
- **AND** it SHALL remain an open obligation in the pending view until answered, closed by the sender's `out`, or the session closes

### Requirement: Pending view

`tala pending` SHALL list all open obligations across the user's sessions: messages with `intent: "req"` that have no correlated reply, plus messages with `expect_reply: true`. Closed sessions SHALL be excluded; reopening a closed session SHALL re-derive its obligations. A `req` answered by an uncorrelated `reply`, or closed by the sender's `out`, SHALL NOT be listed.

Each entry SHALL show the session, sender, message id, a snippet of the message's first text part, and how long the request has been unanswered. A message with no text part SHALL be shown with a placeholder derived from its first non-text part.

#### Scenario: Pending shows unanswered requests
- **WHEN** session `sess_ab12` contains a `req` from `alpha` that no message references via `reply_to`
- **THEN** `tala pending` SHALL list that request with its elapsed time
- **AND** once `beta` sends a `reply` with `reply_to` referencing it, a subsequent `tala pending` SHALL NOT list it

#### Scenario: Pending excludes closed sessions
- **WHEN** an unanswered `req` exists in a session
- **AND** the session is closed
- **THEN** `tala pending` SHALL NOT list it
- **AND** if the session is reopened, `tala pending` SHALL list it again

#### Scenario: Pending snippet uses first text part
- **WHEN** a `req` message contains a text part "please check parse_row" followed by a file part
- **THEN** the pending entry SHALL show the snippet "please check parse_row"

#### Scenario: No text part yields placeholder
- **WHEN** a `req` message contains only a file part
- **THEN** the pending entry SHALL show a `[file]` placeholder instead of an empty snippet

#### Scenario: Pending is empty
- **WHEN** every `req` in every open session has a correlated reply or was closed by its sender's `out`, and no message has `expect_reply: true`
- **THEN** `tala pending` SHALL exit successfully with no entries listed

### Requirement: Pending surfacing in list and status

`tala list` and `tala status` SHALL surface per-session waiting state: for each open session, the number of open obligations and the presence of any active waiter, in both human and `--json` output.

#### Scenario: List shows pending and waiting state
- **WHEN** session `sess_ab12` has 2 unanswered `req` messages and an active waiter
- **THEN** `tala list` SHALL show a column indicating 2 open obligations and an active waiter for `sess_ab12`
- **AND** `tala list --json` SHALL include matching fields per session
