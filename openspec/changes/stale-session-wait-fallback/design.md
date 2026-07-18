## Context

The cross-project eval critic identified one P1 issue: `chit wait` hard-errors with "session not found" when the active session file contains a stale ID. Agent Beta reported this as their most frustrating moment. `cmd_send` already handles this case via a proactive check (lines 787-797 in cli.rs), but `cmd_wait` does not — it passes the stale ID to the daemon and only discovers the problem when the API returns an error (line 1057-1060), where it calls `fail()` which exits the process.

## Goals / Non-Goals

**Goals:**
- `chit wait` recovers from stale active sessions using the same fallback logic as the existing no-active-session path

**Non-Goals:**
- Changing `resolve_session_id` behavior (used by `cmd_recap`, `cmd_close`, `cmd_follow` — those are read/idempotent operations where hard-erroring on stale session is acceptable)
- Changing `cmd_send` behavior (already handles stale sessions)
- Adding new daemon endpoints

## Decisions

### D1: Recovery mechanism — retry loop with reactive catch

- **Decision**: Wrap the session resolution + wait request in a `loop { ... }`. When the daemon returns `SESSION_NOT_FOUND` and the session was sourced from `read_active_session()` (not an explicit `--session` arg), clear the stale session and `continue` the loop to re-resolve.
- **Rationale**: Avoids the consumed-`resp` problem (the error body is consumed before we know we need recovery). A loop naturally handles the "try again with fresh resolution" flow. The wait-all (multiple sessions) branch terminates early via `return`, so the loop doesn't affect it.
- **Alternative considered**: Proactive check like `cmd_send`. Rejected because it adds a round-trip before every `chit wait` call in the happy path. Reactive catch with loop avoids this.
- **Alternative considered**: Extracting a `resolve_or_discover_session()` helper returning `Result<String>`. Rejected because the wait-all branch is terminal (prints and returns `Ok(())`, not a session ID), making the helper's return type incompatible. A loop structure avoids this problem entirely.

### D2: SESSION_NOT_FOUND handling

- **Decision**: Check `resp.status()` BEFORE consuming the body. If not successful, read the error JSON and check for SESSION_NOT_FOUND. If the session was sourced from `read_active_session()` (not explicit `--session`), clear the stale session and `continue` the outer loop. Otherwise, call `fail()` as before.
- **Implementation**:
  ```rust
  let resp = client.get(&url).send().await?;
  if !resp.status().is_success() {
      let err: ErrorResponse = resp.json().await?;
      let from_active = session_arg.is_none();
      if err.error.contains("session not found") && from_active {
          store::clear_active_session().await?;
          continue; // re-resolve from discovery
      }
      fail(json_output, &err.error, "SESSION_NOT_FOUND");
  }
  let result: WaitResponse = resp.json().await?;
  ```

### D3: Spinner re-creation

- **Decision**: Move the spinner creation into the loop body, before the wait URL request. If recovery triggers a second iteration, a new spinner is created.
- **Rationale**: Simple, no extra state needed.
- **Risk**: If recovery happens, the first spinner's dots are on stderr followed by the second spinner's dots. This is acceptable — recovery is rare and the visual artifact is minor.

### D4: Scope — `cmd_wait` only

- **Decision**: Fix only `cmd_wait`. Leave `cmd_follow`/`cmd_recap`/`cmd_close` unchanged (they use `resolve_session_id` which returns stale IDs without validation).
- **Rationale**: `chit wait` is the central blocking primitive for agent-to-agent coordination — the P1 is specifically about `chit wait`. The other commands are read/idempotent operations where a stale-session error is informative (the session was closed, here's the error). `cmd_send` already handles stale sessions proactively.
- **Note**: The proposal is updated to remove `cmd_follow` and `cmd_send` from scope.

## Risks / Trade-offs

- **[Risk]** If the recovered session also gets closed during the recovery window, the API call at line 1050 could still fail. **Mitigation**: Acceptable — this is a race condition that's extremely unlikely and the user can retry. Not worth adding a retry loop.
- **[Trade-off]** Reactive catch means the user sees the HTTP error and recovery messages on stderr. This is acceptable — it provides transparency about what happened.

## Open Questions

None.
