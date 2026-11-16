# Proposal

## Why

The e2e test `status_reports_unavailable_local_context_without_creating_storage` intermittently fails on Windows: after `tala stop` succeeds, deleting `tala.sqlite3` fails with OS error 32 because the file is still open. The durable-daemon spec says stop reports success only after the targeted instance exits, and the client treats release of the daemon's lifetime lock (`daemon.lock`) as that exit. The daemon released the lock while its SQLite connection could still be open: the store sat in shared state that background tasks may still hold, and it was only dropped when the process unwound. Unix permits deleting an open file, so the gap only shows on Windows.

## What Changes

- The daemon closes its SQLite store under the store mutex after the server stops and before it removes daemon metadata and releases the lifetime lock.
- Requests that reach the store after it closes fail with the existing retryable `DAEMON_STOPPING` code.
- The spec states that the lifetime lock is released only after the instance has closed its storage, so a successful stop leaves the database free to move or delete.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `durable-daemon`: Stop linearizability now covers storage release.

## Impact

`src/daemon.rs` and `src/api.rs`. No wire protocol, database schema or CLI surface change.
