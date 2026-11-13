# Proposal

## Why

Indefinite listeners currently inherit a finite three second client deadline, so background agents silently stop listening. Delivery overdue notices can also wake agents that only need inbox activity, and the resume cursor printed after an event can replay it.

## What Changes

- Preserve an indefinite deadline through client transport recovery while keeping positive timeout behavior.
- Add `--only=inbox` to wait, with unrelated inbox and board cursors remaining independently resumable.
- Suppress overdue notifications for recipients that are inactive when the threshold is processed, and report a clear recipient state on send.
- Return an advanced `next_event_after` cursor and use it in the continuation command.
- Keep the existing per-message overdue marker and verify it prevents repeated event generation.

## Capabilities

### New Capabilities

### Modified Capabilities
- `agent-cli`: indefinite waits, filtered wakeups, and exact continuation cursors.
- `agent-attention`: delivery overdue generation and recipient presence reporting.

## Impact

Changes affect CLI wait argument parsing and output, daemon wait snapshots, overdue event processing, and integration/store tests. The RPC shape is extended additively; no incompatible protocol change is expected.
