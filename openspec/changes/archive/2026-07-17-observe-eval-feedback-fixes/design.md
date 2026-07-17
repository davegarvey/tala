## Context

Two observe eval rounds with 4 agents each revealed 8 concrete issues in chit's session management and CLI. The current codebase after the `chit-wait-session-management` change made `chit start` intentionally avoid setting the active session, but eval agents consistently expected it to — leading to session sprawl and cross-project message misrouting. The `SessionSummary` model already carries an optional `name` field, but `chit list` ignores it in default output. Several other small UX gaps were confirmed across multiple independent agents.

## Goals / Non-Goals

**Goals:**
- `chit start` sets the active session so subsequent commands route correctly
- `chit send` with no session context fails gracefully instead of silently creating orphans
- Sessions auto-created from `chit send` inherit the project name from `.chit/config.json`
- `chit list` displays session names in default output
- `chit use` resolves session names (not just opaque IDs)
- `chit init` accepts an optional positional name argument
- `chit session rename` success message renders without JSON quoting
- `chit observe --timeout <secs>` terminates after the given timeout

**Non-Goals:**
- Color coding or compact mode in `chit observe`
- `chit observe --tail` / `--recent` mode
- Renaming `--channel` to `--session-name`
- Breaking existing `--name` flag behavior on `chit init`
- Session editing, threading, or deletion of individual messages

## Decisions

### D1: `chit use` name resolution — daemon-side lookup vs client-side fetch

- **Decision**: Client-side: fetch all sessions from daemon, filter by name match, error if ambiguous or not found.
- **Rationale**: Avoids adding a new daemon endpoint for name→ID lookup. The existing `GET /api/sessions` already returns all sessions with their names. A name lookup is O(n) on session count (typically < 100). Simpler, zero daemon changes.
- **Alternative considered**: Daemon-side dedicated endpoint — cleaner API but unnecessary complexity for this use case.

### D2: `chit send` no-auto-create behavior

- **Decision**: When `chit send` has no `--session` and no active session, list active sessions from the daemon. If none exist, error with "No active sessions. Start one with `chit start`". If one exists, suggest `chit use <id>`. If multiple, list them with IDs/names and suggest `chit use <id>`.
- **Rationale**: Silent auto-creation was the root cause of session sprawl and cross-project pollution. Making it explicit forces the user to be intentional about which session they're sending to.
- **Alternative considered**: Auto-create with project name as session name — we'll do this for `chit start` but `chit send` should be intentional.

### D3: `chit start` sets active session

- **Decision**: Add `store::write_active_session(&session.id)` at the end of `cmd_start`, before the output line.
- **Rationale**: Reverses the previous change based on consistent eval feedback. Agents expect `chit start` → subsequent commands route to the started session.
- **Risk**: If a user runs `chit start` from project A while project B has an active session, project B's active session is overwritten. This is the correct behavior — each project directory has its own `.chit/active-session` file, so they don't interfere.

### D4: Auto-name sessions from project config

- **Decision**: When `chit send` or `chit start` creates a session without an explicit `--name`, read `.chit/config.json` and use the project name as the session name.
- **Rationale**: Named sessions are far more identifiable in `chit list` and `chit observe`. The project name is already configured in `.chit/config.json` (set by `chit init`). This costs nothing and makes session identification dramatically better.
- **Implementation**: `CreateSessionRequest` already has `name: Option<String>` (models.rs:42). In `cmd_start`, pass the `--name` arg through to `CreateSessionRequest.name`. When no `--name` is given, read `store::read_project_config().await` and pass that. In `cmd_send`'s stale-session replacement path, pass the project name from config.

### D5: `chit list` shows session names

- **Decision**: Modify the default output format from `{id}  {status}  {n} msgs` to `{id}  {name or "-"}  {status}  {n} msgs`.
- **Rationale**: The name is the most useful identifier for humans. IDs are opaque and unmemorable. The `--json` output already includes the name.

### D6: `chit init` positional name argument

- **Decision**: Add a positional `name` argument to the `Init` command struct, gated so `--name` and positional don't conflict.
- **Rationale**: The skill doc shows `chit init <name>` as usage, matching user expectation. Both positional and `--name` should work for backward compatibility.
- **Implementation**: Add `name: Option<String>` as the first positional arg. In `cmd_init`, prefer positional if present, then `--name`, then directory name.

### D7: `chit observe` timeout

- **Decision**: Add `timeout_secs` to `ObserveParams` on the daemon side, pass it from the CLI, and implement a server-side timeout that terminates the SSE stream after N seconds.
- **Rationale**: The `_timeout` parameter is currently accepted by the CLI but silently ignored. The daemon's `/api/observe` handler has no timeout support at all — `ObserveParams` only has `since`, `match`, `from`, `channel`. Unlike `wait-new`/`wait-all`/`wait` endpoints which all support `timeout_secs`, observe needs it added. Server-side timeout (rather than client-side) ensures the SSE connection is properly closed.
- **Implementation**: Add `timeout_secs: Option<u64>` to `ObserveParams`. In `observe_events`, use `tokio::select!` to race `rx.recv()` against a `tokio::time::sleep` timer when timeout is set.

### D8: `chit session rename` quoting fix

- **Decision**: Change `result["name"]` to `result["name"].as_str().unwrap_or("")` in the success message.
- **Rationale**: `serde_json::Value`'s `Display` impl renders strings with surrounding quotes. `as_str()` returns the bare string. One-line fix.

## Risks / Trade-offs

- **[Risk]** `chit send` no longer auto-creating sessions could break automated workflows that rely on fire-and-forget. **Mitigation**: The error message clearly suggests `chit start` or `chit use`. Automated flows can adopt `chit start` for explicit session creation.
- **[Risk]** Adding a positional arg to `chit init` could conflict with existing `--name` usage. **Mitigation**: Both positional and `--name` work. Positional wins if both provided (consistent with clap conventions).
- **[Risk]** `chit use` name resolution is client-side and O(n). **Mitigation**: Session count is bounded by practical use (< 100). Performance impact is negligible.
- **[Trade-off]** Reversing the `chit start` active-session behavior means the previous change's intent (no silent switching) is walked back. The eval evidence supports this reversal.

## Resolved Decisions

- **`chit use` name matching**: Exact match only. Partial matches would be ambiguous and error-prone.
- **`chit send` vs `resolve_session_id` inconsistency**: Deliberate. `chit send` is a write operation — a misrouted message is silently lost. `recap`/`close`/`follow` are read/info operations where auto-routing to the sole active session is safe. This distinction is documented and intentional.
- **`chit send` stale session replacement**: When the active session file points to a closed/deleted session, `chit send` creates a new session (this is an edge case recovery path, not a "no active session" path). The new session inherits the project name.

## Open Questions

None.
