## Context

The cross-project eval identified four P1 UX issues in tala 0.25.0's core CLI. The commands `watch`, `wait`, `send --wait`, and `listen` form a family of related but distinct operations (SSE stream, long-poll, send+long-poll, multi-session SSE). Their names don't convey the differences, and two of them (`send --wait`, `watch`) can fail silently with no output during normal operation.

## Goals / Non-Goals

**Goals:**
- Rename `tala watch` to `tala stream` (canonical name), demote `watch` to hidden deprecated alias
- Show progress heartbeat during `tala send --wait` long-poll (repeat timeout + spinner)
- Print a notice when `tala stream --timeout N` exits with no messages (not silent)
- Improve `tala observe` deprecation to error-level stderr message
- Update help text descriptions for `listen` and `stream` to clarify one-session vs all-sessions
- Improve `tala send` error when `--stdin` is missing to mention the flag by name
- Fix `tala status` to verify daemon liveliness via HTTP health check, not just file presence
- Make `tala session rename` idempotent (rename without `--force` just works)

**Non-Goals:**
- Renaming `tala wait` (different semantics from stream — long-poll, not SSE)
- Making `--wait` the default for `tala send` (design preference, not a bug)
- Renaming `tala chat`/`tala send` alias pair

## Decisions

### Rename `watch` → `stream`
- **Why `stream`**: The command produces an SSE event stream. `stream` is a well-known term for this pattern. The old name `watch` overlaps semantically with `wait` (both imply "block and observe"). Status-quo alternative `tala events` was considered but `stream` is shorter and maps to the SSE transport directly.
- **Deprecation pattern**: Follow the same pattern as `observe`→`listen`: `watch` becomes `#[command(hide = true)]`, `stream` becomes the public name with `#[command(alias = "watch")]`.

### Heartbeat for `tala send --wait`
- **Approach**: The current code does a single long-poll GET and blocks. Instead, split the wait into multiple shorter polls (e.g., 30s chunks) with a spinner dot printed between each retry. This prevents the HTTP connection from looking hung and gives progressive feedback.
- **Alternative**: SSE-based wait. Rejected because the `/wait` endpoint already works and is simpler — we just need the client to not appear frozen.

### Watch empty output
- **Approach**: In `cmd_watch`, when the SSE stream ends without producing any message events (only possible events: closed, message, or end-of-stream), print a notice. In text mode: `[no messages received]`. In JSON mode: output an empty JSON array `[]`.
- **Detection**: Track whether any message event was received. After the loop, if count is 0, emit the notice.

### Status health check
- **Approach**: In `cmd_status`, after reading `daemon.json`, make a GET request to `http://{host}:{port}/api/status`. If the request fails, report "daemon not running (stale daemon.json)" instead of "daemon running". This prevents false "daemon running" reports when the daemon has crashed but left its marker file.
- **No daemon.json**: Report "no daemon running" as before (status is inspection-only, does not auto-start).

### Session rename idempotent
- **Approach**: Remove the `force` requirement: `rename_session` in `store.rs` will allow renaming a session regardless of whether it already has a name. The `--force` flag is kept in the CLI for backward compatibility but is ignored.
- **Rationale**: Renaming is an explicit user action — requiring `--force` to perform a rename is surprising and counterintuitive. The operation is already reversible (user can rename again).

## Risks / Trade-offs

- Renaming `watch` to `stream` is a breaking change for scripts. Mitigated by keeping `watch` as a deprecated alias (with a deprecation warning) for the next several releases.
- The `observe` → `listen` rename already went through this; the new pattern is established.
