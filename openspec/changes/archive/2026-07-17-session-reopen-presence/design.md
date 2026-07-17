## Context

The re-eval surfaced four issues:

1. **No session reopen** — Once a session is closed (`closed = true`), the daemon rejects new messages with `409 Conflict: session is closed`. This forced agents to start new sessions, losing conversational continuity.
2. **`chit wait` has no feedback** — When waiting, agents see only a timeout with no indication of pending messages or agent presence.
3. **`chit use` on closed session is misleading** — The error says "No active session named" when the session exists but is closed.
4. **Minor polish** — `close` lacks `--quiet`, `follow`/`observe` naming ambiguous.

## Goals / Non-Goals

**Goals:**
- Add `chit session reopen <id>` to unclose sessions
- `chit use` on closed session shows a clear, actionable error message
- `chit close --quiet` / `-q` to suppress output
- `chit stream` as an alias for `chit follow`

**Non-Goals:**
- Push notifications or webhook delivery (out of scope)
- Read receipts or delivery confirmation (out of scope)
- Full `follow`→`stream` rename (just an alias for backward compat)

## Decisions

### 1. Session reopen
- **Approach**: Add `reopen_session` to Store (sets `closed = false`, updates `last_activity`). Broadcast `DaemonEvent::SessionReopened` on global and per-session channels. New API endpoint `POST /api/sessions/:id/reopen`. New CLI command `chit session reopen <id>` with `--json` support.
- **Rationale**: Minimal change. Reverses close without data loss. Messages sent after reopen get sequential IDs. Broadcasting the event ensures `observe` and `wait` subscribers are notified.
- **Alternative**: Creating a "continuation" session that references the original (over-engineered).

### 2. `chit use` on closed session
- **Approach**: In the daemon's `list_sessions` endpoint, closed sessions are already filtered out by `chit use` when doing name lookup. Fix the error message in the CLI's `cmd_use` when a session ID resolves to a closed session: detect via `chit list --json` or a session status check.
- **Rationale**: Low effort, high clarity impact.
- **Alternative**: New daemon endpoint for session status (overkill for one use case).

### 3. Minor polish
- `close --quiet` — simple bool flag in clap, gate the println
- `stream` alias — add `#[command(alias = "stream")]` to the Follow command struct

## Risks / Trade-offs

- **Reopening a closed session** may surprise users who expect close to be final → Mitigation: print confirmation "Session <id> reopened" to make the state change explicit.
- **`stream` alias** adds another top-level command name → acceptable since `follow` remains and `stream` is more intuitive.
