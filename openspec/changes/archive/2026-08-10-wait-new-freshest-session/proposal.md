# Make `wait --new-session` deliver the freshest never-seen session (B029)

## Why

`tala wait --new-session` is the documented receive-side of the two-agent
handshake ("wait for a session with an incoming message from another agent").
On any daemon with accumulated history (a "busy daemon" — the normal state
after a few agent sessions), the wait fires on **stale unread backlog** instead
of the freshest new session (backlog B029, reconfirmed cycle-09 with an exact
repro):

1. **Stale-beats-fresh.** A session with an old unread incoming message (e.g.
   1.3h old, previous cycle) satisfies the daemon's pre-existing-session scan
   (`find_incoming_session`, added for B003/#44) and is returned instantly.
   A handshake session created seconds later is never seen — the wait already
   exited 0. Cycle-09 repro: beta's wait returned `sess_68s82` (stale question,
   unread) while alpha's fresh handshake `sess_77rna` was created 2s later and
   never delivered.
2. **Already-read messages re-deliver.** The scan ignores the waiter's read
   state entirely. Cycle-09 repro: alpha consumed `sess_77rna` msgs 1–2
   (`history`, cursor 2), then `wait --new-session` re-delivered the
   already-read msg 2 ("chartreuse"). Re-running the wait loops on the same
   stale traffic forever.

Root cause: the scan (api.rs `find_incoming_session`) returns **any** session
with **any** message from another agent, ranking only by message timestamp. It
does not know what the waiter has already seen.

## What Changes

- `/api/sessions/wait-new` gains a `seen` query parameter: a URL-encoded JSON
  map of the waiter's per-session read cursors (session_id → last-seen message
  id). The CLI reads `.tala/cursors.json` (per-session cursors, #46) and sends
  it alongside `sender`.
- `find_incoming_session` becomes read-state-aware. A candidate session must be
  **never-seen by the waiter** (no cursor entry for its id — the waiter has
  never created, sent in, or read it) AND carry an incoming message from
  another agent. Sessions the waiter has a cursor entry for (created, sent in,
  read) are excluded outright: `wait --new-session` is for NEW sessions —
  backlog in known sessions is what `check`/`list`/`wait <sess>` are for.
  Candidates are ranked by freshest incoming message, i.e. "prefer
  freshest/never-seen sessions over oldest unread" (never-seen is the filter;
  unread-in-known-session backlog never satisfies the wait).
- `session create` writes a cursor entry (0) for the new session, so a session
  the waiter itself created is "seen" from birth and never returned as a
  handshake.
- The live event loop is unchanged: a NewMessage from another agent arriving
  while the wait is running always satisfies it (live traffic is fresh by
  definition, and arrives regardless of whether the session was previously
  seen).
- Client-side: `cmd_wait_new` and the `cmd_wait` no-active-session fallback
  pass `seen`.

Resulting behavior:

- Pre-existing session with a genuinely new question (waiter has never seen it)
  → returned immediately (B003 preserved).
- Waiter's own sessions / already-consumed traffic / any session with a cursor
  entry → never returned by the scan (B029 symptom 2 fixed; re-runs never loop
  on stale backlog).
- When both stale-unread and fresh-never-seen sessions exist at wait start →
  the fresh never-seen one wins (B029 symptom 1's deterministic case fixed).
- A live incoming message in ANY session (seen or not) while the wait is
  running still satisfies it via the event loop — nothing is lost.

## Capabilities

### New Capabilities
- *(none)*

### Modified Capabilities
- `receive-new-session`: `tala wait --new-session` now (a) never returns a
  session the waiter has already seen (created/sent-in/read) — backlog in known
  sessions is left to `check`/`list`/`wait <sess>`, (b) among never-seen
  sessions with incoming traffic, returns the freshest. The wait means
  "a NEW session with an incoming message from another agent is ready".
- `session-create`: creating a session records read-state (cursor 0), so
  waiter-created sessions are known-quantities, not future handshakes.

## Impact

- `src/api.rs` — `WaitNewParams.seen`, `wait_new_session` passes it through,
  `find_incoming_session(store, me, seen)` filters+ranks
- `src/cli.rs` — `cmd_wait_new` + `cmd_wait` fallback read `.tala/cursors.json`
  and pass `seen`; `auto_create_session` writes cursor 0
- `tests/e2e.rs` — new tests: consumed message not re-delivered; stale-unread
  loses to fresh-never-seen; waiter-created session never returned as handshake
- Backlog: B029 fixed (with #46 per-session cursors); B021 (delivered/read
  signal) and B030 remain open

## Non-goals

- Per-message delivery/read receipts (B021/M4)
- Waiting on KNOWN sessions via `--new-session` (use `wait <sess>` /
  `check` / `listen` for that)
- `history --limit` tail semantics (B016), "Active session" hint label (B030)
