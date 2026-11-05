# Proposal

## Why

Resuming a conversation after a gap currently requires recovering a thread ID from logs before `tala history` can show the conversation. Listing recent threads lets an agent return to work it can access without guessing message text or searching logs.

## What Changes

- **BREAKING**: Allow `tala history` without a thread ID to list recent visible threads, using a distinct snapshot cursor while preserving `tala history <thread>` behavior.
- Return bounded thread summaries with participants, handoff-only grants, last message, unread and pending counts, kind/project, and a short excerpt in JSON and human output.
- Add indexed transactionally maintained thread summaries and materialized per-agent listing snapshots with stable cursor paging, expiry, and bounded retention.
- Bump the CLI-daemon protocol from 7 to 8 and migrate SQLite schema from 4 to 5.
- Keep visibility narrow: direct participants or direct handoff recipients; board post/reply authors only.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-cli`: support thread listing through `history` without changing full thread reads.
- `agent-messaging`: define visible thread list scope and summary state without acknowledging deliveries.
- `durable-daemon`: support versioned protocol snapshots and stable paginated reads.

## Impact

Affected areas include CLI parsing/rendering, wire protocol models, SQLite migration and transaction updates in `src/store.rs`, and E2E/unit tests. No external dependency is required. Existing `History` RPC behavior remains unchanged; the new list operation requires protocol 8.
