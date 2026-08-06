# Fix `list` active-column clarity (+ empty-history note, reopen selection)

## Why

Backlog B006 (reproduced fresh on v0.25.1 in cycle-04): the word **"active"**
means two different things in a single `tala list` output.

- The **text status column** shows `active` for every non-closed session
  (`let status = if s.closed { "closed" } else { "active" }`, cli.rs:1578).
- The **JSON field** `"active"` means *is-this-my-active-session*
  (`active_session.as_deref() == Some(&s.id)`, cli.rs:1553).

Same row, both meanings: `sess_z4mlp ... active` in text while
`"active":false` in JSON. The text column duplicates the `closed` flag, and the
real active session is already marked with `*`. Related word-overload: the
`resolve_session_id` error "Multiple active sessions: …" also means *open*
sessions (cli.rs:520-525, B022).

Two adjacent clarity defects from the same feedback session, small and safe:

- **B010** — `history` on an empty session prints the header, a blank line, and
  nothing else (exit 0); no "(no messages)" note. Reads like truncated output.
- **B012** — `session reopen` unconditionally calls
  `write_active_session(&id)` (cli.rs:1939), silently moving the `*` marker away
  from the user's working session. Reopen (lifecycle) is conflated with `use`
  (selection). Reproduced: active=`reopen-b2`, reopening an unrelated closed
  session switched active to it.

## What Changes

- **`tala list` text column** (and `session list`): status word becomes
  `open`/`closed` instead of `active`/`closed`. The `*` marker remains the sole
  indicator of the active session. JSON output is **unchanged** — `"active"`
  keeps its machine-consumed meaning (is-my-active-session), and `"closed"`
  already exists for the other meaning; renaming a JSON field would break API
  consumers for no human-facing gain.
- **`resolve_session_id`** error message: "Multiple active sessions" →
  "Multiple open sessions" (B022, same overloaded word).
- **`cmd_recap` text path**: when the session has no messages, print a
  `(no messages yet)` note after the header instead of a bare blank line (B010).
- **`cmd_session_reopen`**: stop calling `write_active_session`; the reopened
  session no longer becomes active. Output becomes
  "Session X reopened (use `tala use X` to make it active)" (B012). Reopen =
  open again; `use` = work here. Existing e2e tests (`test_session_reopen`,
  `test_session_reopen_already_open`) do not assert the active-switch side
  effect and keep passing.

## Capabilities

### New Capabilities
- *(none)*

### Modified Capabilities
- `list-sessions` (text): the status column now reads `open`/`closed`; the
  active session is identifiable only by the `*` marker (as before). JSON shape
  unchanged.
- `recap-session` (text): empty sessions now report "(no messages yet)".
- `session-reopen`: no longer changes the active session; a follow-up `tala use`
  is required to select the reopened session.

## Impact

- `src/cli.rs` — text status word in `cmd_list`; error string in
  `resolve_session_id`; empty-recap note in `cmd_recap`; drop
  `write_active_session` + message wording in `cmd_session_reopen`
- `tests/e2e.rs` — new tests: list text shows `open` (not `active`) for a
  non-closed session and `*` for the active one; history on an empty session
  contains "(no messages yet)"; reopen does not move the active session
- Backlog: B006, B010, B012, B022 fixed; B011/B018 (timeout exit codes),
  B013/B014 (cursor model) remain open

## Non-goals

- Timeout exit-code conventions (B011/B018) — needs a dedicated code decision
- Cursor model rework (B013/B014, per-session cursors) — cycle-05 candidate
- `stream`/`listen` connection banners (B007), `--sender` auth (B004, doc-only)
