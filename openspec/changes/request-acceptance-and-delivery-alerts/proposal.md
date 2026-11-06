# Proposal

## Why

Stored and received messages do not show whether a recipient has committed to a request, and a long-running sender wait cannot currently alert its agent when a message remains unread. Durable acknowledgement and delivery events make silence actionable without claiming that a peer has stopped working.

## What Changes

- Add idempotent `accept` and `decline` operations for incoming requests and optional acceptance deadlines on requests.
- Make acceptance, decline, deadline expiry, and unread delivery overdue visible through `wait` and `pending`; deadline expiry alerts without failing or closing the request.
- Make replies complete requests even when the recipient did not accept first; define cancellation and supersession transitions.
- Give `tala wait` a durable event cursor and block on one daemon-held request; daemon timers create events and survive restart.
- Add receiver attention guidance when unread mail exists and the peer is not currently listening. Do not interpret inactivity as execution failure.
- Defer progress heartbeats and general presence-change events.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-messaging`: request acceptance, decline and opt-in deadlines.
- `agent-attention`: durable lifecycle and delivery events, pending state and wait recovery.
- `agent-cli`: new commands, deadline flags and wait cursors.
- `durable-daemon`: transactionally migrated event state and daemon-side deadline processing.

## Impact

Changes affect SQLite schema and migration, the CLI/daemon wire protocol, request lifecycle logic, blocking wait, pending/attention output, bundled agent guidance, and unit/integration/e2e tests. Existing identities and message records remain in place.
