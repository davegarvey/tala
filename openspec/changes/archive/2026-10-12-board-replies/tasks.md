# Tasks

## 1. Board reply delivery

- [x] 1.1 Create `board_deliveries` with its index on open and insert a delivery for the answered message's author when a board reply is sent (none when the author is the sender); verify with store tests for post reply, self-reply and reply-to-reply
- [x] 1.2 Include unreceived deliveries in the inbox query and set the delivery's `received_at` on consumption; verify with store tests that peek leaves it unread, consumption acknowledges it once for the author only, and other inboxes are unchanged
- [x] 1.3 Count unread deliveries in ambient attention and the attention summary; verify with a store test that a board reply appears as unread with `next` of `tala inbox`
- [x] 1.4 Verify end to end that an author's `inbox --wait` returns a board reply to its post, and that reading the board first leaves it unread

## 2. Waiting for board activity

- [x] 2.1 Generalise the transport-tolerant wait loop so inbox and board waits share it; verify the existing inbox and blocking-send tests still pass
- [x] 2.2 Add `board --wait` with `--timeout`, waiting past the newest existing message when `--after` is absent; verify end to end that a new post wakes the waiter with exit 0, existing posts give exit 3, and nothing is acknowledged

## 3. Documentation

- [x] 3.1 Update `docs/agent-guide.md` and `README.md` for board replies, author delivery, `board --wait` and waiting for a peer to register; verify every new documented command runs
- [x] 3.2 Regenerate the checked-in OpenCode integration with `tala init --refresh` and verify `tala init --check` reports no drift

## 4. Integration

- [x] 4.1 Verify `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `openspec validate --all --strict` pass
- [x] 4.2 Re-run the sibling-worktrees evaluation without hooks and record whether the author sees the reply to its plan

## Workflow follow-up

- Sync the delta spec into `openspec/specs/project-board/spec.md` and archive the change on the feature branch, in the implementation PR.
