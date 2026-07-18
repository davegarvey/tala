## Context

The tala daemon communicates session state to clients via two mechanisms: direct CLI output and SSE events. `DaemonEvent` (in `models.rs`) defines the event types the daemon can broadcast. The daemon writes its PID and socket info to `~/.tala/daemon.json`, but `tala discover` reads from `{project}/.tala/daemon.json`. `cmd_recap()` reads the full transcript but never updates the cursor file. Error messages for daemon-not-found don't distinguish "path doesn't exist" from "daemon failed to start".

## Goals / Non-Goals

**Goals:**
- Renaming a session broadcasts a `SessionRenamed` event over SSE
- `tala discover` finds the daemon at `~/.tala/daemon.json`
- `tala recap` clears unread indicators by writing cursor state
- Daemon-not-found errors report the actual root cause (wrong path vs. startup failure)

**Non-Goals:**
- No new CLI commands or flags
- No changes to the polling-based `tala wait` behavior
- No rework of the SSE subscription model

## Decisions

### D1: Add `SessionRenamed` variant to `DaemonEvent`
**Decision**: Add `SessionRenamed { id: String, old_name: String, new_name: String }` to the `DaemonEvent` enum in `models.rs`.
**Rationale**: Cleanest approach — the existing SSE broadcast infrastructure already handles serialization and dispatch of `DaemonEvent` variants. No new channels or wiring needed.
**Alternative considered**: Polling-based rename detection — rejected because it defeats the purpose of SSE for real-time updates.

### D2: Change discover path from project-local to `~/.tala/daemon.json`
**Decision**: Update the daemon.json lookup in `cmd_discover` to read from `~/.tala/daemon.json` (via `TALA_HOME` env var, defaulting to `~/.tala`).
**Rationale**: The daemon always writes its PID to `~/.tala/daemon.json`. Reading from the project-local `.tala/` is incorrect. Using `TALA_HOME` ensures consistency with the rest of the codebase.
**Alternative considered**: Having the daemon also write a symlink per project — rejected as more complex and fragile.

### D3: Call `write_cursor()` at end of `cmd_recap()`
**Decision**: After successfully reading the full transcript, call `write_cursor()` to mark all messages as read.
**Rationale**: Single-line fix. Users expect reading the transcript to acknowledge messages, matching the behavior of `tala whats-up`.
**Risk**: None — `write_cursor()` already exists and is tested.

### D4: Improved daemon-not-found error with path diagnosis
**Decision**: When daemon connection fails, check if `TALA_HOME`/`~/.tala/daemon.json` exists. If not, emit "Daemon not found at {path}. Check TALA_HOME is set correctly." If the file exists but connection fails, emit the current "daemon failed to start" message.
**Rationale**: Distinguishes between configuration errors and daemon lifecycle errors, giving users actionable next steps.

## Risks / Trade-offs

- Session rename SSE is a new event type — existing SSE clients that don't handle it will silently ignore it (acceptable, graceful degradation)
- Changing daemon.json path could affect users who rely on the broken behavior (unlikely — the broken behavior never worked)
- The `write_cursor` fix may interact with concurrent `tala list` calls — but cursor writes are idempotent and racing writes are already handled
