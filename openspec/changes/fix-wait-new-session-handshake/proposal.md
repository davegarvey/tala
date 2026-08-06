# Fix `wait --new-session` handshake race

## Why

The documented two-agent handshake is: agent A creates a session and asks a
question; agent B runs `tala wait --new-session` to receive it. This flow is
racy and demonstrably fails (backlog B003, reconfirmed cycle-03 as B020):

1. **Misses pre-existing sessions with unread.** `wait --new-session` records
   `existing_count = list_sessions().len()` when it starts and only reacts to
   *future* broadcast events. If the other agent's session (with its question)
   already exists when the wait starts, it is never returned.
2. **Fires on the waiter's OWN `session create`.** The endpoint returns on any
   `SessionCreated` event. When beta creates its own session (e.g. to have a
   session to reply from), the wait returns beta's own session id instead of
   delivering the other agent's question. Reproduced with plain CLI: beta's
   `wait --new-session --json` returned `{"session_id":"sess_yblui"}` — beta's
   own just-created session — while alpha's pre-existing question session went
   undelivered.

The "wait for the other agent's question" promise — the core receive-side flow
for agent-to-agent messaging — therefore does not work in the documented order
of operations.

## What Changes

- `/api/sessions/wait-new` gains an optional `sender` query parameter naming the
  waiting agent (the CLI passes its own agent identity from
  `.tala/config.json`).
- On entry, the endpoint scans **existing** sessions for one that already has at
  least one message from a sender other than the caller; if found, it returns
  immediately with that session id + the first incoming message. This fixes the
  "wait started too late" race without requiring the other agent to do anything
  new.
- While waiting, `SessionCreated` events no longer satisfy the wait when a
  `sender` is provided (a waiter's own session create must not self-trigger).
  Instead, incoming `NewMessage` events whose `sender != caller` satisfy it —
  covering sessions created after the wait starts AND replies arriving in any
  session.
- When no `sender` is provided (old clients / direct API use), behavior is
  unchanged (fires on any `SessionCreated`; the `existing_count` guard on
  `NewMessage` remains).
- Client-side: `cmd_wait_new` and the `cmd_wait` no-active-session fallback pass
  `sender=<agent name>`.

## Capabilities

### New Capabilities
- *(none)*

### Modified Capabilities
- `receive-new-session`: `tala wait --new-session` now (a) returns a session that
  already existed with an incoming message from another agent when the wait
  starts, and (b) ignores sessions the waiting agent itself creates. The wait now
  means "a session with an incoming message from another agent is ready",
  matching the documented handshake.
- `wait-all` / `listen` are unaffected.

## Impact

- `src/api.rs` — `wait_new_session`: pre-existing-session scan + sender-filtered
  event loop
- `src/cli.rs` — `cmd_wait_new` + `cmd_wait` no-active-session fallback pass
  `sender`
- `tests/e2e.rs` — new tests: pre-existing incoming session is returned; own
  session create does not self-trigger the wait
- Backlog: B003/B020 fixed; B021 (delivered/read signal) remains open (feeds M4)

## Non-goals

- Per-message delivery/read state (M4), cross-session `--from <agent>` wait
  (possible follow-up), fixing the cursor/unread model (B014).
