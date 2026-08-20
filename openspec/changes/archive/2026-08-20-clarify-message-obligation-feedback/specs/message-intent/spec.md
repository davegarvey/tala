## MODIFIED Requirements

### Requirement: Intent field on messages

Every `Message` SHALL carry an optional `intent` field with one of four values: `req` (a reply is expected), `fyi` (no reply needed; thread continues), `reply` (this message answers a prior request), or `out` (exchange over; no reply expected).

`tala send` SHALL accept `--intent <req|fyi|reply|out>`. When no `--intent` is given, the CLI SHALL resolve the default by this precedence: `--reply-to` present → `reply`; `--wait` present → `req`; otherwise → `fyi`. When `--reply-to` and `--wait` are both present and no `--intent` is given, the intent SHALL be `reply` and the message SHALL be marked `expect_reply` (see Expectation modifier).

Messages received without an `intent` (older clients, session-creation messages) SHALL be treated as `fyi` on the wire.

The intent SHALL be rendered as a visible tag in `history`, `wait`, `listen`, `check`, and `pending` output (e.g. `[REQ]`), and SHALL be present in `--json` output for all message surfaces.

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

### Requirement: Wait deadline on messages

When `tala send --wait --timeout N` sends a message, the CLI SHALL resolve the effective timeout (the `--timeout` value if given, otherwise the client's configured default) and send it with the message. The daemon SHALL stamp the message with an absolute `waiting_until` timestamp equal to send time plus the resolved timeout. Messages sent without a wait timeout SHALL NOT carry `waiting_until`.

Every public message surface (`history`, `check`, `wait`, `listen`, `pending`) SHALL render `waiting_until` relative to the current time at render time: "waiting, Ns left" while the deadline is in the future, and "wait expired Xm ago" once past, when the request remains unanswered. A request that has been answered by a correlated reply, an applicable uncorrelated reply, or the sender's `out` message SHALL render as settled and SHALL NOT continue to look like an active wait. The original deadline MAY remain available in structured data for diagnostics and history, but it SHALL NOT override the settled presentation.

A message whose deadline has passed and remains unanswered SHALL still render its intent as an open obligation — the deadline expresses urgency, not validity.

#### Scenario: Recipient sees remaining time

- **WHEN** a user runs `tala send --wait --timeout 120 "help"` at 21:34:14
- **AND** the recipient runs `tala check` at 21:34:51 (37 seconds later)
- **THEN** the output SHALL show the message with "waiting, 83s left" (remaining computed at read time, not the original 120s)

#### Scenario: Deadline uses the client's effective timeout

- **WHEN** a client whose configured default timeout is 120 runs `tala send --wait "help"` (no `--timeout`)
- **THEN** the stored message SHALL carry `waiting_until` equal to send time plus 120 seconds, matching the client's actual wait window

#### Scenario: Deadline expired but obligation open

- **WHEN** a message has `intent: "req"` and `waiting_until` in the past
- **AND** no reply or sender `out` message has settled the request
- **THEN** message surfaces SHALL render it as "wait expired" rather than "waiting"
- **AND** it SHALL remain an open obligation in the pending view until answered, closed by the sender's `out`, or the session closes

#### Scenario: Answered request no longer looks active

- **WHEN** a request with a future or expired `waiting_until` is answered by a correlated reply
- **THEN** message surfaces SHALL identify the request as settled or omit its wait marker
- **AND** they SHALL NOT render it as "waiting, Ns left" or as an unresolved active wait
- **AND** `tala pending` SHALL continue to exclude the answered request

## ADDED Requirements

### Requirement: Pending reply guidance is session-targeted

When human-readable `tala pending` output suggests a command for answering an open obligation, the suggested command SHALL include the obligation's session identifier as an explicit `--session` target together with `--reply-to`. This guidance SHALL remain correct when multiple sessions are open or when no active session is selected. JSON output SHALL continue to expose structured session and message identifiers without relying on the human suggestion text.

#### Scenario: Pending suggestion includes its session

- **WHEN** an open request with message id `3` belongs to session `sess_ab12`
- **AND** the user runs `tala pending`
- **THEN** the human-readable entry SHALL suggest an answer equivalent to `tala send --session sess_ab12 --reply-to 3`

#### Scenario: Pending guidance is safe with multiple sessions

- **WHEN** multiple open sessions exist and one contains an unanswered request
- **THEN** the suggested reply command SHALL target the request's session explicitly
- **AND** the user SHALL NOT need to rely on the project's active-session marker to answer that request
