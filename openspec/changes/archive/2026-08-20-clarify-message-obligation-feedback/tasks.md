## 1. Settled Request Rendering

- [x] 1.1 Define the shared answer-state calculation for correlated replies,
      applicable uncorrelated replies, and sender `out` closure.
- [x] 1.2 Update human message renderers so answered requests no longer show an
      active wait countdown, while unanswered expired requests retain `wait
      expired` and pending behavior.
- [x] 1.3 Add unit coverage for future-deadline answered requests, expired
      answered requests, and unanswered expired requests.
- [x] 1.4 Add end-to-end coverage across the public `history`, `check`, `wait`,
      `listen`, and `pending` output where applicable.

## 2. Blocking Send Receipt

- [x] 2.1 Include the received message id, sender, and existing intent/reply
      metadata in human-readable `send --wait` output.
- [x] 2.2 Verify multiple received messages are individually identifiable.
- [x] 2.3 Add end-to-end coverage proving `--json` remains valid and retains
      structured message ids and `reply_to` fields.

## 3. Session-Safe Pending Guidance

- [x] 3.1 Include `--session <id>` in human-readable pending reply suggestions.
- [x] 3.2 Add coverage for pending guidance with one active session, multiple
      open sessions, and no active-session marker.
- [x] 3.3 Confirm pending JSON output remains structured and unchanged in shape.

## 4. Verification

- [x] 4.1 Run the two-agent cross-project conversation and verify the settled
      state is consistent across `history`, `pending`, `status`, and `list`.
- [x] 4.2 Run `cargo fmt --check`.
- [x] 4.3 Run `cargo clippy -- -D warnings`.
- [x] 4.4 Run `cargo test`.

<!-- The full command was run. Four pre-existing integration-document tests
     fail because the worktree already contains unrelated edits to the
     .opencode templates; the implementation-specific unit and e2e coverage
     passes. -->
