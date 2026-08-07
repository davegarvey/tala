---
schema: spec-driven
created: 2026-08-07
---
# Surface the active daemon home; warn on default-home fallback (B036)

## Why

Backlog B034 ("phantom session-create id", tracked since cycle-12) was finally
root-caused in cycle-14: it is NOT a daemon race. A stray daemon (started
06:55Z with `HOME=/workspace` and no `TALA_HOME`) owns the DEFAULT home
`$HOME/.tala`. Any `tala` invocation that runs **without** `TALA_HOME` set
silently connects to that daemon instead of the shared one. Sessions created
there (sess_mrs5i, sess_ky7vo) appear to "vanish" from the shared daemon's
view — exactly the B034 symptom reproduced across three cycles.

The failure is compounded by zero visibility:

- `tala status` (running case) prints PID/Port/Host/Since but NOT the home dir.
  The no-daemon case prints "checked <home>/daemon.json" but the running case
  hides which home the daemon lives in.
- `status --json` (running) has no `home` field.
- No command warns when `TALA_HOME` is unset and the daemon is being used from
  the default home — so two daemons silently split the agent fleet's view.

For agent-to-agent messaging, silently operating on a different message store
is a trust failure: a message "sent" to a named session may go to a daemon
nobody else reads. This is the top P1 for cycle-14 (B036).

## What Changes

- `tala status` (text): add a `Home: <path>` line showing the resolved home
  dir (annotated `(from TALA_HOME=...)` when the env var is set).
- `tala status --json`: add `home` field (and `tala_home_set: true/false`) to
  both the running and not-running responses.
- `cmd_status`: when `TALA_HOME` is unset and the daemon is running, print a
  one-line stderr warning that the default home is in use:
  `warning: TALA_HOME is not set — using default daemon home <path>`.
- `ensure_daemon_running`: when about to (re)use an existing daemon found via
  the default home (TALA_HOME unset), no change to behavior; the warning lives
  in status so it never pollutes command stdout/stderr in normal operation.
- No change to daemon-side storage or APIs.

## Capabilities

- Agents can now instantly tell which daemon/home a command talks to.
- The multi-daemon split becomes visible instead of a "phantom session" ghost.

## Impact

- `status` output changes (additive text line + JSON field) — existing tests
  assert substrings (`daemon running`, `PID:`), so they keep passing.
- No wire/API changes; no data migration.
