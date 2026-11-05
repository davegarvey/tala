# Tasks

## 1. Protocol and CLI contract

- [x] 1.1 Add the protocol 8 `ListThreads` operation and versioned CLI list mode; verify `history <thread> --after` remains unchanged and invalid cursor-mode combinations fail before RPC.
- [x] 1.2 Add `INVALID_CURSOR` to documented error metadata and human/JSON error behavior; verify help lists it and cursor errors use the same public fields.

## 2. Indexed state and migration

- [x] 2.1 Migrate schema 4→5 with summary, role, ordered access projection, unread, open-request, snapshot, row, and cursor tables; verify a v4 fixture preserves messages and receipts and backfills canonical counts.
- [x] 2.2 Maintain thread summaries, ordered access rows, conversation roles, direct handoff grants, unread counts, and open-request rows in the send, inbox, reply, resolve, and supersede transactions; verify each state transition and migration parity in store tests.

## 3. Snapshot creation and paging

- [x] 3.1 Materialize all visible per-agent rows and high-water state atomically using the agent-leading activity index and indexed access queries; verify direct participant, handoff-only, board author/replier, and hidden same-project visibility.
- [x] 3.2 Add opaque versioned cursor mapping, keyset continuation, 15-minute expiry, eight-snapshot eviction, and generic invalid-cursor handling; verify ordering, replay, wrong-identity, malformed, expiry, and eviction cases.

## 4. CLI output and regression coverage

- [x] 4.1 Render JSON and escaped human thread summaries with participant/grant roles, bounded excerpt, project, unread, and fresh/stale pending counts; verify JSON/human parity and cross-project board context.
- [x] 4.2 Add E2E coverage for snapshot stability under new messages, receipt updates, request resolution, and visibility/name changes; verify listing never acknowledges delivery.
- [x] 4.3 Add indexed query-plan or seeded large-history verification showing list creation avoids full message-table aggregation.

## 5. Integration and documentation

- [x] 5.1 Run formatting, clippy, full tests, and OpenSpec validation; sync and archive the completed change and verify no active change remains.
