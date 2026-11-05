# Tasks

## 1. Refresh outgoing Git context

- [x] 1.1 Update outgoing context construction to copy invocation-time worktree, branch, revision, and canonical working directory while preserving the registered project; verified with the focused E2E test.
- [x] 1.2 Extend E2E coverage for a sender registered in one linked worktree and posting from a nested directory in another branch/revision; verified the message stays on the canonical project board and records invocation metadata.
- [x] 1.3 Extend E2E coverage for posting from outside Git after registration in a Git checkout; verified the project remains registered and Git fields are absent.

## 2. Sync specifications and validate

- [x] 2.1 Sync the `agent-messaging` and `project-board` delta specs into the main specs, preserving existing scenarios; verified the archive comparison reports no remaining delta.
- [x] 2.2 Run `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, and `openspec validate --all`; verified all complete successfully.

## Workflow follow-up

- Archived the change after syncing delta specs; verified `openspec validate --all` passes with no active changes left.
