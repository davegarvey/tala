## Why

The cross-project eval critic identified four actionable issues from two autonomous agents testing tala. Sessions can close without explicit user action (risk of data loss), `tala start` without `--wait` gives no delivery indication, the `--file` flag on `tala send` creates false expectations of file transfer, and the `--new-session` flag is undiscoverable from the top-level help. These undermine confidence in the session model and block smooth UX for new users.

## What Changes

- **Session auto-close fix**: Investigate and fix the cause of sessions closing without explicit user action. Likely root cause: daemon idle timeout (10 min default) shuts down the daemon, dropping all in-memory sessions. Fix: increase default idle timeout, add session persistence, or prevent daemon shutdown when sessions exist.
- **Delivery indication on start**: When `tala start` runs without `--wait`, indicate whether any agents are actively listening so the user knows their message was queued and will be delivered.
- **Rename `--file` to `--message-file`**: Rename the `--file` flag on `tala send` to `--message-file` (or `--from-file`) to clarify it reads message content from a file, not attaches a file.
- **Surface `--new-session` in top-level help**: Add mention of `tala wait --new-session` to the top-level `tala --help` summary so users can discover this workflow entry point.

## Capabilities

### New Capabilities
- `session-auto-close-fix`: Investigation and fix of sessions closing without user action

### Modified Capabilities
- `cli-ux`: Delivery indication on `tala start`, `--file` renamed to `--message-file`, `--new-session` surfaced in top-level help

## Impact

- `src/cli.rs`: `cmd_start()` output changes, `--file` flag rename, help text updates
- `src/daemon.rs` or `src/store.rs`: Idle timeout or session persistence changes
- `tests/e2e.rs`: Tests for new behaviors
