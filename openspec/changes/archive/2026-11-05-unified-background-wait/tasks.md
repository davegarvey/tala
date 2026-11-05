# Tasks

## Unified wait behavior

- [x] Add `tala wait` with an indefinite default, finite timeout, separate inbox and board cursors, and an explicit project option.
- [x] Poll inbox non-consumingly and the board independently; deduplicate board replies also present in the inbox.
- [x] Return IDs, event types, sources, page indicators, and exact fetch/restart commands without message bodies.
- [x] Add the atomic project-scoped board `latest_id` and reject a daemon response that lacks it.

## Specifications and verification

- [x] Sync the change into `agent-cli`, `agent-attention`, and `project-board` specs, then archive this change.
- [x] Add end-to-end coverage for direct messages, correlated replies, board posts and replies, deduplication, non-consumption, explicit projects, timeout, baseline, and separate cursor pagination.
- [x] Run formatting, the full test suite, clippy, and OpenSpec validation.
