## Context

Three eval rounds of the cross-project scenario revealed friction in chit's session management. The core issues cluster into two areas:

1. **Session lifecycle**: `chit start` performed two actions — create session AND set active — but did so silently, confusing agents who expected to control which session was active.
2. **Wait semantics**: `chit wait` required either an explicit `--session`, an active session file, or exactly one open session. Any other state produced an error. This forced agents into an extra command sequence (`list` → `use` → `wait`) for a blocking operation.

The eval framework itself had reliability issues: daemons dying on timeout, missing CHIT_HOME in task prompts, and feedback lost via file writes.

## Goals / Non-Goals

**Goals:**
- `chit wait` blocks and returns messages in all session states (0, 1, 2+)
- `chit start` creates only — no side effects on active session
- Agents always see what `chit wait` is doing via initial status messages
- Eval setup completes reliably; feedback is never lost
- All existing tests pass without modification (except those testing removed behavior)

**Non-Goals:**
- Not changing `chit send` auto-create semantics (intentional design)
- Not adding `chit reply` or similar new commands
- Not changing the `--new` flag (still available for explicit use)
- Not adding session filtering/priority to wait-all

## Decisions

### D1: `chit start` — create only, no active session side effect
**Decision**: Remove `write_active_session()` call from `cmd_start`. Users who want the new session active use `chit use <id>`.
**Rationale**: Eval feedback showed silent switching was confusing. Separate create from activate.
**Alternative considered**: Print a warning when switching. Rejected because "no side effects" is simpler and more predictable.

### D2: `chit wait` — adapt to session count
**Decision**: `chit wait` with no args resolves the target session in three tiers: explicit `--session` → active session file → daemon session list (0=wait-new, 1=per-session, 2+=wait-all).
**Rationale**: This removes all error cases. In every state, `chit wait` either returns messages or blocks until messages arrive.
**Alternative considered**: Keep erroring with suggestions (eval feedback showed agents found this frustrating).

### D3: Wait-all as a daemon endpoint, not client-side merge
**Decision**: New `/api/sessions/wait-all` daemon endpoint subscribes to `global_tx` broadcast and returns the next `NewMessage` as a `WaitResponse`.
**Rationale**: Cleaner than client-side SSE parsing of the observe stream. Reuses existing broadcast infrastructure. Returns standard `WaitResponse` format.
**Alternative considered**: Client-side concurrent per-session waits with `tokio::select!`. Rejected because it requires the `futures` crate and handles dynamic session counts poorly. Client-side SSE parsing of `/api/observe`. Rejected because observe streams forever and lacks timeout parameter.

### D4: Eval feedback as inline Task results
**Decision**: Agents return feedback in their final Task message, not by writing to a file.
**Rationale**: Task results are always delivered to the orchestrator. File writes can silently fail (wrong CWD, missing directory, no permission).

### D5: `nohup + disown` for daemon lifecycle
**Decision**: Both setup functions use `nohup "$CHIT_BIN" daemon > /dev/null 2>&1 &` followed by `disown`.
**Rationale**: Without disown, the bash tool's process group kill on timeout also terminates the daemon.

### D6: Auto-create notification to stdout
**Decision**: `eprintln!("Created session {}", session.id)` → `println!("→ Created session {}", session.id)` (guarded by `!json_output`).
**Rationale**: Agents watching stdout see it. JSON output is unaffected (session_id is already in the response body).

### D7: Eval task templates with personas
**Decision**: Agents get a role ("developer maintaining project-alpha"), eval context ("your real job is to evaluate chit"), and a command reference to explore from.
**Rationale**: Previously agents followed a narrow 3-step script and never touched most commands. Personas + exploration goals produce richer feedback.

## Risks / Trade-offs

- [Wait-all misses messages during race] → The broadcast channel buffers events. If a message arrives between the session list query and wait-all subscription, it's picked up by the re-check logic (same pattern as per-session wait).
- [Wait-all returns messages from any session, not just the one the agent cares about] → Intentional design. The output includes `[sess X]` so the agent can route replies. In practice, agents loop: `wait` → process → `wait` again.
- [Agents don't know about `chit use`] → The task templates list it in the command reference. The status messages from `chit wait` also show which session is active.
- [Removing auto-active from `chit start` is BREAKING] → True, but the behavior was untested in eval (agents used `--session` explicitly) and was the source of confusion.
