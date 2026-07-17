## Why

Two eval rounds revealed friction in chit's session management and wait behavior. `chit start` silently switched the active session, causing confusion. `chit wait` errored when no sessions existed instead of adapting. The eval framework itself had reliability issues (daemon death, lost feedback) that reduced iteration speed.

## What Changes

- **BREAKING**: `chit start` no longer sets the active session. Use `chit use <id>` explicitly.
- **BUG FIX**: `chit start "message"` no longer duplicates the initial message.
- `chit wait` now adapts to session count: 0 sessions → wait-new, 1 session → per-session wait, 2+ → wait-all (new endpoint merges across sessions).
- `chit wait` prints initial status messages (e.g. "Waiting for new messages in session X...") so the user knows what mode it's in.
- `chit wait` sets the active session when it receives messages.
- New daemon endpoint `/api/sessions/wait-all` — returns the next message from any session.
- `chit send` auto-create notification moved from stderr to stdout (with `→` prefix).
- Eval framework: daemon lifecycle fixed with `nohup + disown`, CHIT_HOME exported in task templates, feedback collected inline via Task results instead of file writes.
- Eval task templates rewritten with agent personas and open-ended exploration goals.

## Capabilities

### New Capabilities
- `wait-all`: New daemon endpoint that subscribes to global events and returns the next message from any session. Used by `chit wait` when 2+ sessions exist.

### Modified Capabilities
- `session-lifecycle`: `chit start` no longer auto-sets active session. `chit use` is the explicit mechanism.
- `message-waiting`: `chit wait` now adapts to 0, 1, or 2+ sessions with status messages. New `--new` flag remains for explicit use but is no longer required.
- `message-sending`: Auto-create notification on stdout (not stderr). Duplicate message on `chit start` fixed.

## Impact

- `src/cli.rs` — `cmd_start` (no active session write, no duplicate send), `cmd_wait` (multi-session resolution, status messages, active session write), `cmd_send` (auto-create notification to stdout)
- `src/api.rs` — new `wait_all` handler, new route
- `src/store.rs` — unchanged
- `src/models.rs` — unchanged
- `tests/e2e.rs` — updated tests for new `chit start` behavior
- `eval/run.sh` — daemon lifecycle, task templates, feedback collection
- `.opencode/skills/chit-eval/SKILL.md` — updated lessons
