# Persist message history across daemon restarts

## Why

A daemon restart destroys every transcript. The daemon persists only session
metadata (`sessions.json`); the messages `HashMap` and per-session `next_msg_id`
live solely in RAM (`Store`). Cycle-06 reproduced this end-to-end on main
(v0.25.1, rebuilt from `caf8718`): two agents exchanged 7 messages in
`sess_f9sxx`, then `tala stop` (graceful SIGTERM, daemon prints "daemon
stopped") → auto-restart → `tala history` prints "(no messages yet)",
`tala list` shows `0 msgs`, `tala check --json` → `{"cursor":0,"messages":[]}`.
Session metadata (name, timestamps) survives; the transcript is gone.

This is B024 (P1, cycle-05, reconfirmed cycle-06). It is destructive and
silent: no warning that a restart loses history, and it happens on the
24h idle timeout and on crash, not just explicit `stop`. Agent-to-agent
messaging without durable transcripts means the shared conversation memory
is erased on any daemon lifecycle event. `tala` cannot be a message bus for
agents if the bus forgets everything whenever it restarts.

## What Changes

- **Persist messages.** New `messages.json` in `tala_home()` holding
  `{ messages: HashMap<session_id, Vec<Message>>, next_msg_id: HashMap<session_id, u64> }`,
  written atomically (tmp + rename, same pattern as `persist_sessions`).
- **Persist on every message add** (`Store::add_message`), not just on shutdown:
  a `kill -9`/crash then loses at most the in-flight message, not the whole
  history. Writes are atomic and message volume is low (agent chat), so the
  per-send fsync-equivalent cost is acceptable.
- **Load on daemon start.** `Store::load_persisted()` additionally loads
  `messages.json`: restores messages per session and resumes `next_msg_id`
  (stored value; fallback = max message id + 1 for hand-edited/legacy files).
  If a session exists in `sessions.json` but has no message entries, it loads
  with an empty transcript (exactly the current behavior, so no surprise).
- **Graceful shutdown persists both files** (existing `persist()` call sites).
- Backward compatible: absence of `messages.json` → empty state, daemon behaves
  exactly as today (old installs lose nothing new; they just get persistence
  from now on).

## Capabilities

### New Capabilities
- `transcript-durability`: transcripts survive daemon restart (`stop`/`start`,
  idle-timeout shutdown, crash up to the last persisted message).
- `message-id-continuity`: per-session message ids resume after the last
  persisted id — no id reuse, no client-visible cursor confusion after restart.

### Modified Capabilities
- `daemon-persistence`: `persist()` now writes messages + next-ids in addition
  to session metadata; `load_persisted()` restores them.
- `messaging`: sending a message is now a durable operation (acknowledged send
  implies "survives restart").

## Impact

- **Backlog:** fixes B024 (P1). Related family (B014/B023/B025) already fixed
  in open PR #46 (per-session cursors, unmerged); this change is orthogonal —
  store-level persistence vs. read-model. Branches both touch `src/store.rs`
  but disjoint regions (persist/load vs. cursor helpers) and `tests/e2e.rs`
  (new tests append). Conflicts, if any, are mechanical.
- **Files:** `src/store.rs` (persist/load/add_message), `src/daemon.rs`
  (load already called; persist already called — likely no change),
  `src/models.rs` (none), `tests/e2e.rs` (new restart-persistence tests).
- **Performance:** one atomic JSON write per sent message; negligible at
  agent-messaging volume. No new dependencies (serde_json/tokio already used).
- **Risks:** none identified beyond write amplification; messages.json is
  bounded by transcript size, same as memory.
