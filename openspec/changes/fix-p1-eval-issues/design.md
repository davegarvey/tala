## Context

Four P1 issues from the cross-project eval loop affect the `tala` CLI's `send`, `wait`, `list`, and help text. All live in `src/cli.rs` (the CLI handler layer). No API, model, or store changes needed — the daemon already correctly rejects closed-session writes (409). The fixes are ergonomic: better error messages, clearer help, initial wait feedback, and a missing fallback in `compute_session_unread()`.

## Goals / Non-Goals

**Goals:**
- Closed-session send errors should be actionable (suggest reopen/start)
- `tala wait` should print immediate feedback with the timeout value
- Reduce default `wait` timeout from 300s to 60s
- Fix `unread_count` to exclude own messages even when no project config exists
- Clarify `wait`/`stream`/`listen` help text so users know which command to pick

**Non-Goals:**
- No API changes (daemon endpoints stay the same)
- No data model changes
- No new CLI subcommands

## Decisions

1. **Closed-session UX improvement**: Keep the existing API 409 response. In `cmd_send()`, when `SESSION_CLOSED` is detected, print "Session X is closed. Use `tala session reopen X` to reopen it or `tala start` to create a new one." Also clear the stale active session so auto-resolution picks the right thing next time.
2. **Wait initial feedback**: Already partially present (eprintln lines in `cmd_wait()`). Enhance to include the timeout value: "Waiting for messages in session X (timeout: Ys)...". The spinner (`.` dots every 5s) stays.
3. **Default timeout**: Change the hardcoded default from 300 to 60 in both `read_user_config()` default and in the API `wait_for_message()` fallback. Users who want longer can set `default_timeout` in config.
4. **unread_count fix**: In `compute_session_unread()`, when `read_project_config()` returns `None`, fall back to `get_default_sender()`. This ensures the sender name used by `get_sender_name()` is the same name used for filtering.
5. **Help text**: Add a Usage section to `Wait`, `Stream`, `Listen` help that briefly says when to use each. Keep it short.

## Risks / Trade-offs

- [Changing default timeout] → Existing scripts relying on 300s default will timeout sooner. Mitigation: configurable via `default_timeout` in global config.
- [unread_count fallback] → If directory name doesn't match the sender name, the filter still doesn't work. Mitigation: `tala init` remains the recommended approach; the fix only helps the common case where both resolve to the directory name.
