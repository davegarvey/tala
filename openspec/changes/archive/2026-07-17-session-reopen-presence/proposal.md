## Why

A re-eval after shipping session lifecycle fixes revealed a new set of friction points. Agents were frustrated by `chit wait` timing out with zero feedback about the other agent's presence. Closed sessions cannot be continued, forcing new sessions and fragmenting conversation history. `chit use` on a closed session gives a misleading "No active session" error instead of clarifying it's closed. Minor inconsistencies (`close` lacks `--quiet`, `follow` vs `observe` naming ambiguous) add up.

## What Changes

- **NEW**: `chit session reopen <id>` — reopens a closed session (sets `closed = false`), allowing continued messaging
- **IMPROVED**: `chit wait` shows pending session/message count before blocking; clearer timeout output
- **IMPROVED**: `chit use` on a closed session errors with "Session '<id>' is closed. Use `chit session reopen` to continue"
- **POLISH**: `chit close --quiet` / `-q` flag to suppress confirmation (consistent with `chit send`)
- **POLISH**: `chit follow` gets a `chit stream` alias for clarity

## Capabilities

### New Capabilities
- `session-reopen`: Reopen closed sessions via `chit session reopen` and API endpoint

### Modified Capabilities
- `cli-ergonomics`: Better error message for `chit use` on closed sessions; `--quiet` on `chit close`; `stream` alias for `follow`

## Impact

- `src/cli.rs` — `cmd_session_reopen`, `cmd_close` (--quiet), `cmd_follow` (stream alias)
- `src/api.rs` — new `POST /api/sessions/:id/reopen` endpoint
- `src/store.rs` — `reopen_session()` method, `set_closed(false)`
- `src/models.rs` — `ReopenSessionRequest` / `ReopenSessionResponse` if needed
- `tests/e2e.rs` — new tests for reopen, close --quiet, use on closed
