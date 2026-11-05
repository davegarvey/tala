# Proposal

## Why

Tala 1.8.0 can deadlock when a daemon dies during `tala wait`: the waiter's concurrent inbox and board requests contend for the startup lock, and a blocking lock call prevents the lock owner from progressing. The stalled CLI prevents agents from receiving messages or restarting the daemon without manual process cleanup.

## What Changes

- Make daemon startup-lock acquisition yield while another client starts or recovers the daemon.
- Bound lock contention and recheck daemon health before attempting startup.
- Add a regression test for daemon death during a long-lived wait and recovery by a concurrent inbox command.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `durable-daemon`: concurrent clients recover from daemon loss without blocking the event loop or holding startup coordination for a long-lived wait.

## Impact

Affected code: `src/client.rs` and daemon lifecycle end-to-end tests. No wire protocol or database changes are expected. Recovery must preserve the current rule that an unreachable but potentially live daemon is never replaced.
