# Proposal

## Why

An indefinite `tala wait` started by an older binary can retain the shared startup lock after an upgrade. Newer commands then time out before reaching the daemon, and an agent cannot safely identify or terminate another agent's process.

## What Changes

- Move current startup coordination and stop barriers to a new lock-file generation so a process holding the legacy lock cannot block upgraded clients.
- Keep the daemon lifetime lock as the cross-version guard that prevents two daemons from owning one Tala home.
- Add regression coverage for startup while the legacy lock is held.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `durable-daemon`: upgraded clients can recover from startup locks held by processes from older releases.

## Impact

Changes affect local daemon startup/stop coordination in `src/client.rs` and `src/daemon.rs`, lifecycle comments, daemon integration tests, and the durable-daemon spec. The CLI and wire protocol remain unchanged.
