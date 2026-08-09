## 1. Message intent metadata

- [x] 1.1 Add `intent` enum field (`req`/`fyi`/`reply`/`out`, default `fyi`), `reply_to: Option<u64>`, `expect_reply: bool`, `waiting_until: Option<DateTime<Utc>>` to `Message` and `SendMessageRequest` in `src/models.rs`; add `wait_timeout: Option<u64>` to `SendMessageRequest`
- [x] 1.2 Add `intent` handling to the send endpoint in `src/api.rs`: validate values, treat missing intent as `fyi`; stamp `waiting_until` (send time + `wait_timeout`) when present; validate `reply_to` resolves within the target session
- [x] 1.3 Add `--intent <req|fyi|reply|out>`, `--reply-to <id>`, `--expect-reply` flags to `tala send` in `src/cli.rs`; CLI resolves defaults by precedence (`--reply-to` → `reply`, `--wait` → `req`, else `fyi`; `--reply-to`+`--wait` → `reply` + `expect_reply`); reject invalid intent values and invalid modifier combinations; resolve and pass effective timeout for `--wait`
- [x] 1.4 Render intent badges (`[REQ]` etc.) and `[REPLY→id]` in `history`, `wait`, `stream`, `listen`, `check`; include `intent`, `reply_to`, `expect_reply`, `waiting_until` in all `--json` message output

## 2. Wait deadline (waiting_until)

- [x] 2.1 Stamp `waiting_until` in the send endpoint from the client-supplied `wait_timeout`
- [x] 2.2 Render relative remaining time ("waiting, 83s left" / "wait expired 2m ago") in all message surfaces including `pending`, computed at read time; expired deadlines keep the obligation open

## 3. Reply correlation + pending view

- [x] 3.1 Answer-derivation in the store: a request is answered when a later same-session message has `reply_to` referencing it, OR an uncorrelated `intent: reply` answers the oldest unanswered `req` in the session; a sender's `out` closes their own open requests
- [x] 3.2 Pending-obligation query in the store: unanswered `req`s plus `expect_reply` messages, excluding closed sessions
- [x] 3.3 Add `tala pending` command in `src/cli.rs` with JSON and human output (session, sender, message id, snippet, elapsed)
- [x] 3.4 Surface per-session open-obligation count and active-waiter presence in `tala list` and `tala status` (human column + JSON fields)

## 4. Waiters registry + identity + warnings

- [x] 4.1 Add `ActiveWait` struct and `Mutex<Vec<ActiveWait>>` registry to `src/store.rs` (scope: session id or "any new session"; since; deadline; identity); prune expired entries at registration
- [x] 4.2 Register/deregister waits in `src/api.rs` wait endpoints (scope guard on exit; failed stream send deregisters)
- [x] 4.3 Overlap detection helper: same session, or either scope is "any new session"; suppress new-session↔new-session pairs
- [x] 4.4 Add `--sender` flag to `tala wait` and `wait --new-session`; pass identity on wait requests from `src/cli.rs`
- [x] 4.5 Pre-flight overlap warning: daemon returns overlapping waits with the wait response; CLI prints non-blocking note (name, scope, remaining time) before waiting; cross-scope wording notes the new-session wait will not receive that session's content
- [x] 4.6 In-wait overlap event: notify existing waiter (one line) when a new overlapping wait (non-new-session-pair) registers
- [x] 4.7 Timeout hint computed client-side: after a wait timeout and at wait registration, check sessions with unread messages from other senders (client cursor); print "N session(s) with an unread message (sess_ab12) — run `tala check`"

## 5. SSE-backed wait + delivery receipt

- [x] 5.1 Convert the wait path to an SSE stream with typed events (`message`, `overlap`, `hint`, `result`); preserve blocking/timeout/cursor behavior
- [x] 5.2 In `--json` mode, buffer stream events and emit a single `WaitResponse` document on completion (preserve script contract)
- [x] 5.3 On event-channel lag (`RecvError::Lagged`), re-sync from the store since cursor instead of dropping events
- [x] 5.4 Print delivery receipt ("✓ sent (msg N)") in `cmd_send` with `--wait` before entering the wait state

## 6. Docs + eval baseline

- [x] 6.1 Update `.opencode/skills/tala/SKILL.md` with the intent protocol (flags, pending view, waiting visibility)
- [x] 6.2 Update CLI help text for `send`, `wait`, and `pending` documenting new flags and warnings
- [x] 6.3 Verify the V6 deadlock fix mechanically: `wait --new-session` with a pre-existing unread session now prints a hint naming the session (covered by `test_wait_new_session_timeout_hint`). Full agent re-run of the intent-v6 scenario (342s baseline) deferred to a post-merge eval loop.

## 7. Tests

- [x] 7.1 Unit tests: intent default precedence and validation, waiting_until stamping from effective timeout, reply_to same-session validation, answer-derivation (correlated, uncorrelated, out), pending query, overlap detection and suppression, registry pruning
- [x] 7.2 E2E tests: intent badges in output, relative deadline rendering, strict wait returns only the correlated reply, pending listing and closed-session exclusion, pre-flight warning, timeout hint, single-document `--json` wait output, lag re-sync, registry entry removed on disconnect
