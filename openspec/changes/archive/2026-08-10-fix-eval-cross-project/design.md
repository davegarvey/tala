## Context

Four fixes from the cross-project eval critic. Sessions live entirely in-memory behind the daemon process, which has a 600s (10 min) idle timeout by default. If the daemon shuts down, all sessions are lost — this is the likely cause of "sessions auto-closing." The remaining issues are CLI UX: delivery indication on `tala start`, `--file` flag naming, and `--new-session` discoverability.

## Goals / Non-Goals

**Goals:**
- Prevent sessions from being lost due to daemon idle shutdown
- Add delivery-awareness output to `tala start` when used without `--wait`
- Rename `--file` to `--message-file` on `tala send`
- Surface `--new-session` in top-level `tala --help` summary

**Non-Goals:**
- Full session persistence to disk (beyond what's needed to survive daemon restarts)
- Heartbeat/presence indicators (deferred to future change)
- Command surface consolidation (deferred to future change)
- Help output reorganization (deferred to future change)

## Decisions

1. **Session auto-close fix**: The daemon's idle timeout defaults to 600s (10 min) via `read_user_config()`. Increase default to 86400s (24h) so sessions survive normal work sessions. Additionally, persist open sessions to `~/.tala/sessions.json` so they survive daemon restarts. On daemon startup, reload any persisted sessions and mark them as still open. On daemon shutdown (graceful), persist open sessions. This prevents data loss from accidental restarts or idle timeouts.

2. **Delivery indication**: In `cmd_start`, after printing the session ID and "Message sent as...", query the daemon for active subscribers/sessions. Print "→ No agents currently listening" or "→ 2 agents listening" based on subscriber count. This uses the existing `/api/sessions` endpoint to check for open sessions from other agents.

3. **Rename `--file`**: Rename the clap arg from `file` to `message_file` with `long = "message-file"`. Keep `file` as a hidden alias for backward compatibility, with a deprecation warning if used. Update the help text to clarify it reads message content from a file.

4. **Surface `--new-session`**: Add an `after_help` note to the `Wait` command variant and/or add a line to the top-level `#[command]` `long_about` or `after_help` mentioning that `tala wait --new-session` waits for a new session to be created.

## Risks / Trade-offs

- [Session persistence adds complexity] → Keep the persistence layer minimal: JSON serialization of session metadata only (no messages). Messages remain in-memory.
- [Idle timeout increase could cause daemons to accumulate] → The daemon already exits on idle; increasing the timeout just extends the grace period. Users can still set `idle_timeout` explicitly in config.
- [--message-file is a flag rename] → Keep `--file` as a hidden alias with deprecation warning for backward compatibility, then remove in a future version.
