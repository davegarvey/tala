## Why

Four P1 state-consistency bugs erode user trust: session renames don't propagate to listeners, `tala discover` uses the wrong daemon.json path, `tala recap` doesn't clear unread indicators, and daemon-not-found errors mislead users into thinking the daemon itself is broken rather than a path/config issue.

## What Changes

- **Session rename → SSE broadcast**: Add `SessionRenamed` variant to `DaemonEvent` and emit it on rename so SSE-based consumers (stream, listen) see the update
- **Fix `tala discover` daemon.json path**: Change daemon.json lookup from `{project}/.tala/daemon.json` to `~/.tala/daemon.json`
- **`tala recap` clears unread counts**: Call `write_cursor()` at end of `cmd_recap()` to mark messages as read
- **Better daemon-not-found error**: When daemon can't be found, check if path exists and give actionable guidance (e.g., "TALA_HOME points to /path/which/does/not/exist")

## Capabilities

### New Capabilities

- `session-events`: SSE event broadcasting for session lifecycle changes (rename, etc.)

### Modified Capabilities

- `daemon-discovery`: Fix daemon.json path resolution; improve error messages when daemon cannot be found
- `message-read-state`: Ensure `tala recap` properly acknowledges messages as read
- `session-management`: Emit rename events through the daemon event system

## Impact

- `cli.rs`: modify `cmd_recap`, `cmd_discover`, daemon-not-found error handling
- `models.rs`: add `SessionRenamed` variant to `DaemonEvent`
- `daemon.rs` or session handler: emit `SessionRenamed` event on rename
- No API or breaking changes
