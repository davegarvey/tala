## Why

The cross-project eval critic identified one P1 issue: `chit wait` fails with "session not found" when the active session file (`.chit/active-session`) contains a stale ID pointing to a closed or nonexistent session. Agent Beta reported this as their most frustrating moment and one-thing-to-change. Agent Alpha also highlighted the seamless daemon model but inadvertently confirmed the brittleness — when active session state is correct, chit feels great; when it's stale, the user hits a hard error with no recovery path.

The core flow of agent-to-agent collaboration depends on `chit wait`. A stale session file breaks this flow entirely, forcing the agent to manually run `chit list`, identify the correct session, and `chit use` it before retrying. For autonomous agents, this is a workflow blocker.

## What Changes

- `chit wait` with a stale active session no longer hard-errors — it falls back by clearing the stale session, discovering available sessions from the daemon, and either:
  - Using the sole active session if exactly one exists
  - Waiting for a new session if none exists
  - Listing multiple sessions with a suggestion to `chit use`

## Capabilities

### Modified Capabilities

- `session-lifecycle`: `chit wait` handles stale active sessions gracefully with automatic fallback.

## Impact

- `src/cli.rs` — `cmd_wait` (add stale session recovery in the error path)
- `tests/e2e.rs` — add tests for stale session recovery
