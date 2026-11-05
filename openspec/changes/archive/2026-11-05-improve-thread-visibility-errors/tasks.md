# Tasks

## 1. Unify visibility failures

- [x] 1.1 Add a shared non-disclosing `THREAD_NOT_VISIBLE` failure and use it for history, handoff/continued direct threads, message status, and board replies; verify all call sites return the same code, message, hint, and `next` command.
- [x] 1.2 Add e2e coverage for visible access, hidden and missing thread parity, and inaccessible handoff/board reply/message status; verify no owner handle or foreign project details appear.
- [x] 1.3 Document recovery with the original identity or `tala handoff`; verify the guidance uses no private handle.

## 2. Sync and validate the specification

- [x] 2.1 Sync both `agent-cli` and `agent-messaging` deltas into their main specs and verify the resulting specs with `openspec validate --specs`.
- [x] 2.2 Run formatting, clippy, the full test suite, `openspec validate --all`, and `git diff --check`; verify all pass.

## Workflow follow-up

- Archive this OpenSpec change on the feature branch before opening the PR.
