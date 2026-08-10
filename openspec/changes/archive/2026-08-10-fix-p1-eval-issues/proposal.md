## Why

Four P1 issues were identified in the cross-project eval: messages sent to closed sessions produce confusing errors instead of actionable guidance; the listen/stream/wait naming is the top onboarding confusion; `tala wait` gives no initial feedback and defaults to a 300s timeout that feels broken; and `unread_count` in `tala list --json` counts the user's own sent messages, misleading workflow automation.

## What Changes

1. **Closed-session send UX**: When `tala send` detects the target session is closed, suggest reopening or starting a new session instead of a terse error. Clear stale active session state.
2. **CLI help clarity**: Improve help text for `wait`, `stream`, `listen` to make the distinction obvious — which to use when. Add cross-reference hints.
3. **`tala wait` initial feedback and timeout**: Print a clear "Waiting for messages in session X (timeout: Ys)..." message immediately. Reduce the default timeout from 300s to 60s.
4. **`unread_count` bug fix**: `compute_session_unread()` falls back to `get_default_sender()` when project config is missing, so own messages are properly excluded.

## Capabilities

### New Capabilities
- *(none — all fixes modify existing capabilities)*

### Modified Capabilities
- *(no spec-level requirement changes; all fixes are implementation/UX improvements)*

## Impact

- `src/cli.rs`: `cmd_send()`, `cmd_wait()`, `compute_session_unread()`, CLI help strings
- `src/store.rs`: potentially `read_project_config()` or new helper
- No API or model changes required
