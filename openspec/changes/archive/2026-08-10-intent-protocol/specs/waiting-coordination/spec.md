## Purpose

Live visibility of waiting state in tala: the daemon tracks who is waiting on what, warns when waits overlap (deadlock prevention), hints when unread messages exist, and delivers wait wake-ups and events over SSE — so agents never wait blind.

## ADDED Requirements

### Requirement: Waiters registry

The daemon SHALL track every active wait: its scope (a specific session, or "any new session"), the time the wait started, its deadline (start + timeout), and the identity of the waiting client (self-reported sender name). A wait SHALL be registered when the wait begins and removed when it ends (reply received, timeout, or error).

The registry SHALL prune entries whose deadline has passed whenever a new wait is registered, so crashed or disconnected clients cannot leave stale entries. A wait whose stream fails mid-wait SHALL be deregistered.

Two waits SHALL be considered overlapping when both target the same session, or when either targets "any new session".

#### Scenario: Wait registers and deregisters
- **WHEN** a client starts `tala wait --session sess_ab12 --timeout 60`
- **THEN** the daemon SHALL record a wait with scope `sess_ab12` and a deadline 60 seconds after registration
- **AND** when the wait returns (message arrives or timeout), the entry SHALL be removed

#### Scenario: Expired entries are pruned
- **WHEN** a client disconnects without deregistering, leaving a registry entry whose deadline has passed
- **AND** a new wait is registered
- **THEN** the stale entry SHALL NOT be reported to the new waiter

### Requirement: Waiting client identity

Wait requests SHALL include the waiting client's identity (the sender name from project config or `--sender`). The identity is self-reported and informational only — it SHALL be recorded with the wait entry so warnings can name the waiter.

#### Scenario: Identity attached to wait
- **WHEN** a client whose sender name is `project-alpha` starts a wait
- **THEN** the registry entry SHALL record the identity `project-alpha`

### Requirement: Wait stream events

Waits SHALL be delivered over a persistent SSE stream rather than a single long-poll response. The stream SHALL carry typed events: `message` (a new message matching the wait), `overlap` (a new overlapping wait registered), `hint` (timeout or unread content notice), and a terminal `result` event with the wait outcome (messages, timeout flag, cursor).

In `--json` mode, the CLI SHALL buffer stream events and emit a single `WaitResponse` JSON document on completion, preserving the existing wait contract for scripts. Human-readable notes (warnings, hints) SHALL go to stderr in human mode and SHALL be included as typed events in JSON mode.

#### Scenario: Wait stream carries events
- **WHEN** a client waits on `sess_ab12` and a new overlapping wait registers mid-wait
- **THEN** the client's stream SHALL deliver an `overlap` event naming the new waiter before any subsequent message event

#### Scenario: JSON mode buffers to single document
- **WHEN** a client runs `tala wait --json` and receives messages during the wait
- **THEN** the CLI SHALL print exactly one `WaitResponse` JSON document on completion, containing all received messages

### Requirement: Pre-flight overlap warning

When a wait is registered and the registry already contains an overlapping wait, the daemon SHALL emit an `overlap` event to the new waiter before the wait proceeds, naming the other waiter, its scope, and its remaining deadline. The wait SHALL proceed regardless. Overlaps between two "any new session" waits SHALL NOT produce warnings (the normal multi-agent startup pattern). For cross-scope overlaps (session wait vs new-session wait), the warning SHALL state that the new-session wait will not receive the other session's content.

#### Scenario: New-session wait warns about an existing session wait
- **WHEN** a client is waiting for a reply on `sess_ab12` with 13s left
- **AND** a second client starts `tala wait --new-session`
- **THEN** the second client SHALL print a note such as "alpha is waiting on sess_ab12 (13s left) — this wait will not receive that session's messages"

#### Scenario: New-session waits do not warn each other
- **WHEN** two clients are both running `tala wait --new-session`
- **AND** the second starts its wait
- **THEN** neither SHALL print an overlap warning

#### Scenario: Non-overlapping waits produce no warning
- **WHEN** a client is waiting on `sess_ab12`
- **AND** a second client starts a wait on `sess_xy9`
- **THEN** the second client SHALL NOT print an overlap warning

### Requirement: In-wait overlap event

While a wait is active, if a new overlapping wait (excluding new-session↔new-session pairs) is registered, the daemon SHALL deliver an `overlap` event to the existing waiter naming the new waiter and its scope.

#### Scenario: Existing waiter notified of new waiter
- **WHEN** a client is waiting on `sess_ab12`
- **AND** a second client starts waiting on `sess_ab12`
- **THEN** the first client SHALL receive an `overlap` event naming the second client

### Requirement: Timeout hint for unread content

The CLI SHALL compute the unread-content hint client-side using its own cursor state: when a wait ends by timeout (or when a wait is registered), the CLI SHALL check for sessions containing messages from other senders with ids after the client's cursor, and SHALL print a hint naming each such session: "1 session exists with an unread message (sess_ab12) — run `tala check`". The hint SHALL be suppressed in `--json` mode except as a typed stream event.

#### Scenario: Timeout hints at unread session
- **WHEN** a client runs `tala wait --new-session --timeout 300`
- **AND** session `sess_ab12` exists with a message from another sender that the client has not read
- **THEN** the client SHALL print a hint naming `sess_ab12` when the wait times out

#### Scenario: Registration-time hint
- **WHEN** a client starts `tala wait --new-session`
- **AND** session `sess_ab12` already exists with an unread message from another sender
- **THEN** the client SHALL print the hint before entering the wait

### Requirement: SSE-backed waiting with lag recovery

`tala wait` and `tala send --wait` SHALL receive messages and events over the wait stream. When the stream detects missed events (channel lag), the client SHALL re-sync by reading the session's messages since its cursor from the store, so no message is silently dropped.

#### Scenario: Message during wait arrives immediately
- **WHEN** a client is waiting on `sess_ab12`
- **AND** a message arrives in `sess_ab12` one second into the wait
- **THEN** the wait SHALL return with that message promptly without waiting for any poll interval

#### Scenario: Lagged stream re-syncs
- **WHEN** the daemon's event channel drops events while a client waits (burst of messages)
- **THEN** the client SHALL re-sync from the store and still deliver the missed messages in the wait result

### Requirement: Delivery receipt for send --wait

When `tala send --wait` sends a message, the daemon SHALL confirm receipt (message stored) before the client begins waiting, and the CLI SHALL print the confirmation so the sender knows the message was accepted.

#### Scenario: Send --wait confirms receipt
- **WHEN** a user runs `tala send --wait "help"`
- **THEN** the CLI SHALL print a confirmation that the message was stored (e.g. "✓ sent (msg 12)") before showing the waiting state
