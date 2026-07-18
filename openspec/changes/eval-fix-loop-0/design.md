## Context

The eval loop found that the core session model has lifecycle bugs and UX gaps. All state is in-memory only — sessions, names, and activity are lost on daemon restart. Error messages and help text need polish.

## Goals / Non-Goals

**Goals:**
- Fix session reopen so it actually works (sets closed=false, allows subsequent operations)
- Fix daemon resolution error messages to respect `$TALA_HOME` ordering
- Add default 300s timeout to `tala listen`
- Persist session names so they survive daemon restarts
- Improve `tala stop` message when daemon is already dead
- Add `tala session close` subcommand
- Improve help text for listen/stream, --file, tala init

**Non-Goals:**
- Full persistence of all session state (messages, etc.)
- Renaming the `tala chat` / `tala send` command
- Deprecating `tala init`

## Decisions

- **Session rename persistence**: Write session names to `{tala_home()}/sessions.json` as a simple map of session_id → name. On daemon startup, load this file. On rename, update both in-memory and on-disk. This is simpler than SQLite or a full event log.
- **Reopen fix**: The `reopen_session` handler correctly sets `session.closed = false`, but the issue may be that `send_message` checks `session.closed` and rejects — verify the check is consistent.
- **Listen default timeout**: Use 300s matching the existing `default_timeout` in user config. Check user config first, then fall back to 300.
- **Daemon stop message**: Check if daemon.json exists before trying to kill; if not, print "daemon is not running" instead of attempting kill.

## Risks / Trade-offs

- [Sessions.json file] → If two daemons run concurrently (unlikely), they'll race on writes. Acceptable because daemon is singleton on a machine.
- [Listen default timeout] → 300s may be too short for long-running workflows. Users can override with `--timeout 0` for no timeout or `--timeout N` for custom.
