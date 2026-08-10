## Why

`tala session close <id>` (and `tala close <id>`) of the session that is currently the project's ACTIVE session leaves dangling state: the `.tala/active-session` file still points at the now-closed session, `tala list` renders a `*` marker on the closed row, `tala use` reports the closed session as "Active session", and a bare `tala send "msg"` then fails with "Session is closed" (exit 1).

For autonomous agents the active-session marker is the primary targeting mechanism. A stale marker silently corrupts every subsequent bare `send`/`wait` until the agent manually runs `use --clear` — a workflow blocker discovered by both alpha and beta agents in the cycle-08 feedback session. The top-level `tala close` (no arg) already clears the marker correctly ("Active session was closed and cleared"); the close-alias paths do not, because `cmd_close` computes `was_active = session_arg.is_none() && read_active_session()==Some(id)` and the `session close` alias always passes `Some(id)`.

## What Changes

- `cmd_close` computes `was_active` from the resolved session id alone: `read_active_session() == Some(&session_id)`, regardless of whether the session was addressed positionally, via `-s`, or through the `session close` alias.
- When the closed session was active, the active-session file is cleared and `list` no longer shows the `*` marker on the closed row (existing clear + messaging logic, now reached from all close paths).
- JSON output keeps `"active_cleared": true` when applicable.
- `use` after such a close behaves as with no active session set (shows available sessions).

## Capabilities

### Modified Capabilities

- `session-lifecycle`: closing the active session clears the active-session marker from every close entry point (`close`, `close <id>`, `session close <id>`), matching the behavior of `close` with no argument.

## Impact

- `src/cli.rs` — `cmd_close` (~2 lines: `was_active` computation)
- `tests/e2e.rs` — new e2e tests: close active via alias clears marker; close active via explicit id clears marker; close non-active leaves marker untouched; `close` no-arg path unchanged
- No daemon-side changes; no API changes; no schema changes.
