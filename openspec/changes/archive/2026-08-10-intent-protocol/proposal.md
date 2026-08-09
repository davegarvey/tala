## Why

Six eval experiments (10 agent evaluations, all transcripts in `eval/results/intent-v0..v6/`) showed agent conversations work but get disjointed: participants can't tell whether a reply is expected, whether more messages are coming, or when the exchange is done. Agents independently requested intent declaration, and a deliberate deadlock baseline (V6) measured a 342-second window where both agents waited blind, neither knowing the other was waiting. Sessions provide topic scoping, but waiting state is invisible.

## What Changes

1. **Message intent metadata** — every message carries an optional intent: `req` (reply expected), `fyi` (no reply needed), `reply` (answers a request), `out` (exchange over). Set via `tala send --intent <tag>`; `--wait` implies `req`; `--reply-to` without `--intent` implies `reply`. Rendered as a badge in all message surfaces. (**BREAKING**: `--wait` defaults change, message JSON gains fields)
2. **Reply correlation** — messages can carry `reply_to: <message-id>` (`--reply-to <id>`, interpreted within the same session), making `send --wait` wait for the reply to that specific message instead of any message. An uncorrelated `reply` falls back to answering the oldest unanswered request in the session; a sender's `out` closes their own open requests.
3. **Wait deadline** — `send --wait --timeout N` stamps an absolute `waiting_until` timestamp on the message (computed from the client's resolved effective timeout); recipients see *remaining* time ("23s left" / "expired 2m ago") computed at read time, never a stale duration. The obligation outlives the deadline (expired ≠ cancelled).
4. **Pending view** — `tala pending` (and a pending/awaiting column in `list`/`status`) surfaces requests awaiting reply: "who owes whom".
5. **Waiters registry** — the daemon tracks active waits (scope, since, deadline, identity) and warns on overlap: pre-flight note when a new wait collides with an existing waiter, one-line in-wait events, and a timeout hint when unread sessions/messages exist. Scoped by session (= topic); `wait --new-session` is global scope. Expired entries are pruned; new-session↔new-session overlaps are suppressed to avoid warning storms.
6. **Wake-on-write** — `wait`/`send --wait` become SSE-backed instead of long-poll requests, carrying wait events (overlap warnings, hints) as typed stream events. (**BREAKING**: the `/wait` response wire format changes from a single JSON document to a stream; `--json` mode buffers to a single `WaitResponse` document to preserve the contract)

## Capabilities

### New Capabilities

- `message-intent` — intent tags, reply correlation, wait deadline, and derived pending view
- `waiting-coordination` — waiters registry, overlap warnings, timeout hints, in-wait events, SSE-backed waiting

### Modified Capabilities

- *(none — no existing requirement changes; all behavior is additive)*

## Impact

- `src/models.rs` — `Message`, `SendMessageRequest` gain `intent`, `reply_to`, `expect_reply`, `waiting_until`, `wait_timeout`
- `src/api.rs` — wait endpoints register/deregister with the registry; send stamps deadline; SSE-backed wait with typed events; re-sync from store on lag
- `src/store.rs` — active-wait registry with pruning, pending-obligation query
- `src/cli.rs` — `--intent`, `--reply-to`, `--expect-reply`, `--sender` (on wait) flags; pending command; warnings/hints rendering; client-side unread hint computation
- `src/daemon.rs` — registry initialization
- `eval/` — re-run `intent-v6` deadlock scenario as before/after baseline (342s → target <60s)
- `.opencode/skills/tala/SKILL.md` — document the intent protocol
