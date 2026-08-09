# Fix broken read-model: per-session cursors replace the single global cursor

## Why

The read/unread model compares ONE global cursor (`.tala/cursor`, a single u64)
against PER-SESSION message ids (`next_msg_id` is a per-session
`HashMap<String, u64>` — every session's ids start at 1). Comparing a global
number to per-session id spaces is fundamentally unsound, and cycle-05
reconfirmed all three failure directions on a FRESH daemon (v0.25.1, rebuilt
from main):

1. **B014 — under-reporting (blocks the core receive flow).** After beta's
   cursor was 5, alpha sent one message to a brand-new session (per-session
   id 1). Beta's `tala list` shows `1 msgs` with NO `(1 new)`, `tala check
   --json` returns `{"cursor":5,"messages":[]}` — the fresh message is
   invisible to both `list` and `check`. A new session's low ids are always
   below a cursor that any other session inflated.
2. **B023 — `send` writes the per-session id of the message you just sent
   into the global cursor** (cli.rs:972), silently inflating it for all
   sessions and guaranteeing future under-reporting.
3. **B025 — `history` on an empty session resets the global cursor to 0.**
   `cmd_recap` writes `recap.cursor.unwrap_or(0)` (cli.rs:1534); empty
   sessions have `cursor: None` → 0 → *everything* re-marked unread
   (`sess_dkk8n ... 7 msgs (4 new)` for messages already read and replied to).
   Over-reporting whiplash, the mirror image of B014.

The promise "new messages are visible to list/check and reading one session
does not affect another" is the foundation of agent-to-agent messaging; it
currently fails in every direction.

## What Changes

- **Per-session cursor store.** Replace `read_cursor()/write_cursor()` (single
  u64 in `.tala/cursor`) with a per-session map persisted as JSON in
  `.tala/cursors.json` (`{session_id: last_seen_id}`):
  - `read_cursors() -> HashMap<String, u64>`
  - `read_cursor(session_id) -> u64` (entry lookup, default 0)
  - `write_cursor(session_id, cursor)` (merge + persist)
  - The legacy `.tala/cursor` file is ignored (its value cannot be attributed
    to any session); it is left in place, not deleted.
- **`cmd_send`**: write the cursor for `msg.session_id` only (B023).
- **`cmd_recap` (history)**: write the cursor for the session being read; an
  empty session writes nothing instead of 0 (B025).
- **`cmd_list` / `cmd_status`**: per-session unread computed from each
  session's own cursor (B014).
- **`cmd_whatsup` (check)**: fetch per session since that session's cursor;
  after printing, update each session's cursor to that session's max id.
  JSON output keeps `"cursor"` (max of per-session cursors, backward compat)
  and adds a `"cursors"` map; text output reports how many read markers were
  updated instead of a single meaningless number.
- **`cmd_listen` / observe**: when no explicit `--since` is given, the client
  passes a per-session since map (`since_map`, URL-encoded JSON) to
  `/api/observe`; the server replays each session since that session's cursor
  (`map.get(sid).unwrap_or(since)`). Explicit `--since` keeps global replay
  semantics. This fixes listen's variant of B014 (a new session's low ids were
  never replayed).
- `compute_session_unread` / `compute_total_unread` take a cursor per session
  instead of one global value.

## Capabilities

### New Capabilities
- `per-session-read-state`: unread/read tracking is independent per session;
  reading, checking, or sending in one session can no longer change another
  session's unread state (M4 partial: per-session read markers are now
  persistent project state in `.tala/cursors.json`).

### Modified Capabilities
- `list-sessions` / `status`: `(N new)` / `total_unread` now reflect real
  per-session unread (fresh sessions with low ids are no longer invisible).
- `check-messages`: operates per session; JSON adds `"cursors"`; `"cursor"`
  retained as max for backward compatibility.
- `history-session` / `recap`: marks only the read session; empty sessions no
  longer corrupt global state (no state to corrupt — there is no global state).
- `send-message`: marks the target session read through that session's cursor.
- `listen-all` (no `--since`): replays per session since that session's cursor.

## Impact

- `src/store.rs` — cursor store: file path, read/write API, map semantics
- `src/cli.rs` — `cmd_send`, `cmd_recap`, `cmd_list`, `cmd_status`,
  `cmd_whatsup`, `cmd_listen`, `compute_session_unread`,
  `compute_total_unread`
- `src/api.rs` — `observe_events` accepts `since_map`
- `tests/e2e.rs` — new tests: per-session unread independence (B014),
  empty-history no-reset (B025), send no-inflate (B023), check per-session
  cursors, listen replays new-session messages without `--since`
- Backlog: B014, B023, B025 fixed; B024 (transcript persistence) and
  B011/B018 (timeout exit codes) remain open

## Non-goals

- Transcript persistence across daemon restarts (B024 — messages are still
  in-memory only; separate change)
- Timeout exit-code conventions (B011/B018 — decision recorded in backlog:
  dedicated exit code 3; implementation deferred)
- `--sender` impersonation auth (B004)
