## Context

Two eval agents tested chit and reported three P1 UX issues. The CLI has two command pairs (`observe`/`follow`) that confuse users, `chit wait` without active session errors unhelpfully, and there's no way to discover other agents. All changes are in the Rust codebase (`src/cli.rs`, `src/api.rs`, `src/models.rs`, `src/store.rs`, `tests/e2e.rs`, `README.md`).

## Goals / Non-Goals

**Goals:**
- Rename `observe` to `listen` (keep `observe` as hidden deprecated alias) and `follow` to `watch` (keep `follow` and `stream` as hidden deprecated aliases). Emit deprecation warning stderr when alias used.
- Make `chit wait` without args and multiple open sessions list sessions instead of auto-targeting. Keep blocking wait-new behavior for 0 sessions.
- Add `chit agents` command to list unique senders across open sessions.
- Update all help text, README, and embedded SKILL.md references.
- Daemon API routes keep current paths (`/api/observe`, `/api/sessions/:id/events`).

**Non-Goals:**
- No changes to the daemon's internal routing or broadcast model.
- No full peer-to-peer agent discovery (still single-daemon).
- No invitation protocol.

## Decisions

1. **Hidden aliases with deprecation warnings**: Old names are hidden from help output but still work. Using them prints a stderr warning. This gives users time to migrate without breaking scripts.

2. **`chit wait` improvement**: Only change the 2+ sessions case (list instead of auto-pick). Keep 0-session blocking (wait-new) and 1-session auto-target unchanged for backward compat.

3. **Agent discovery via `chit agents`**: Query open sessions for unique sender names. New `GET /api/agents` daemon endpoint aggregates this. `AgentSummary { sender, last_seen, message_count }` model.

4. **No new API routes**: `cmd_listen` calls `/api/observe`, `cmd_watch` calls `/api/sessions/:id/events`. Avoids changing the daemon API contract.

## Risks / Trade-offs

- [Agent discovery is session-scoped, not agent-scoped] → sufficient for single-daemon model
- [Closed sessions excluded from agents] → agents with only closed messages are hidden; users must reopen or recap
