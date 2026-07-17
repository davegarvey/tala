## Red Team Report

### P0 — Must Fix

**1. `chit use` on closed session: design approach is underspecified**
The design says "detect via `chit list --json` or a session status check" but `cmd_use` currently sets the active session file locally without querying the daemon about a session's closed status. The name lookup already excludes closed sessions (`list_sessions` returns all sessions, but the name lookup in `cmd_use` uses the existing `resolve_session_id` flow which uses `list_sessions` and filters for active only). For an explicit session ID, `chit use sess_abc` would blindly set it as active even if closed. The spec needs a concrete approach: either query `GET /api/sessions/:id` to check the closed state before setting, or move the validation to the daemon.

**2. Missing `--json` on `chit session reopen`**
Every other `chit session` subcommand (`rename`, `show`, `close`, `list`) supports `--json`/`-j`. Reopen should too. Missing scenario: reopen with `--json` returns `{"session_id": "sess_abc", "status": "reopened"}`.

**3. No event notification on reopen**
`close_session` broadcasts `DaemonEvent::SessionClosed` to subscribers. Reopening a session should broadcast a corresponding event (e.g. add `SessionReopened` variant to `DaemonEvent`) so that agents waiting on `chit wait` or watching via `chit observe` are aware the session is active again.

**4. `DaemonEvent` model missing `SessionReopened` variant**
Without this, `chit observe` won't show reopen events and `chit wait` won't be notified. Add `SessionReopened(String)` variant matching the existing pattern.

### P1 — Should Fix

**5. "Use by name on closed session" scenario is contradictory**
The THEN says `chit use my-session` should error with "Session 'my-session' is closed..." but the parenthetical note says "name lookup ignores closed sessions, so this falls through to 'No active session named...'". These contradict each other. Pick one behavior: either name lookup finds closed sessions too (and shows the reopen message), or it doesn't (and shows the generic error). The parenthetical stating the latter is "acceptable" undermines the requirement.

**6. `chit close --quiet` with `--json` interaction**
When both `--quiet` and `--json` are set, should JSON output be suppressed? Current `--json` output contains structured data that programs consume — suppressing it would break callers. The spec should clarify that `--quiet` only suppresses human-readable output, not JSON.

### P2 — Polish

**7. Missing: no "already open" event on reopen**
The "Reopen an already-open session" scenario says succeed silently. Should it still broadcast a `DaemonEvent`? Probably not (no state change). This is fine as-is but could surprise agents watching via `observe` if they see a reopen event for an already-open session.

**8. `chit stream` alias not in the cli-ergonomics spec**
The alias is in the proposal and design but doesn't appear in the cli-ergonomics spec. Should add a requirement with scenario there.
