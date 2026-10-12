# Tasks

## 1. Evaluation baselines

- [x] 1.1 Add `eval/scenarios/interrupted-wait.md` reproducing #93 (daemon connection dropped during `send --wait`) and verify it fails against 0.34.0 as reported
- [x] 1.2 Add `eval/scenarios/shared-checkout.md` with two agents in one directory and verify against 0.34.0 that a copied public ID lets one agent consume the other's inbox
- [x] 1.3 Add `eval/scenarios/sibling-worktrees.md` and verify against 0.34.0 that board posts are not shared across worktrees

## 2. Protocol and error contract

- [x] 2.1 Bump `PROTOCOL_VERSION` to 3 and verify the existing mismatch test reports `PROTOCOL_MISMATCH` against a protocol 2 envelope
- [x] 2.2 Add `retry`, `stored` and `next` to `Failure`, set them for every daemon error, and verify with unit tests per error code
- [x] 2.3 Define the error-code list in one place, render it in `--help`, and verify a test fails if a code is emitted that the list lacks
- [x] 2.4 Add exit code 4 to `main` and the help text, and verify with a CLI test

## 3. Transport classification

- [x] 3.1 Make liveness three-way (live, refused, unreachable) and spawn only on refused or absent metadata; verify with tests using a closed port and a non-routable timeout
- [x] 3.2 Return `DAEMON_UNREACHABLE` with the OS reason and `next: tala status`; verify no second daemon process starts in the unreachable test
- [x] 3.3 Verify by test that a transport failure during recipient resolution never yields `RECIPIENT_NOT_FOUND`

## 4. Resilient wait and idempotent sends

- [x] 4.1 Expose `inbox --reply-to=<id>` and verify it waits only for that correlated reply
- [x] 4.2 Retry transport errors in `wait_inbox` with capped backoff until the deadline; verify the interrupted-wait scenario returns the reply with exit 0
- [x] 4.3 Return exit 4 with `stored` and `next` when a wait fails permanently after storage; verify with a test that keeps the daemon unreachable
- [x] 4.4 Derive `auto:` keys in the CLI and implement the daemon-side deduplication window; verify blind resend, after-window repeat, explicit-key conflict and two-sender scenarios
- [x] 4.5 Update `docs/agent-guide.md` on retries, exit codes and `--reply-to`, and verify every documented command runs

## 5. Identity handles

- [x] 5.1 Generate `th_` handles in `register`, store credentials under hashed handle paths with mode 0600, and verify an `agt_` value for `--agent` fails with `IDENTITY_ERROR`
- [x] 5.2 Implement `register --reissue` that succeeds once and retires the ID-keyed credential; verify the second reissue fails and the inbox is intact
- [x] 5.3 Verify by test that no listing, history or search output contains a handle
- [ ] 5.4 Update the agent guide for per-session registration and keeping the handle in working notes, and verify the shared-checkout scenario passes using instructions only

## 6. Output defaults and project key

- [x] 6.1 Make JSON the default output, add `--human`, keep `--json` as a no-op; verify existing CLI tests pass with JSON default and render tests use `--human`
- [x] 6.2 Key projects on the git common directory, add `worktree` to context, and verify the sibling-worktree scenario and a separate-repositories test
- [x] 6.3 Document the output and project-key changes in the agent guide and README, and verify the examples

## 7. Attention

- [x] 7.1 Add the `Attention` operation with a single-transaction query and verify it acknowledges nothing
- [x] 7.2 Compute `age_seconds` for returned messages and stale counts against the 24-hour threshold; verify fresh, stale and unregistered-counterpart cases
- [x] 7.3 Make bare `tala` return the summary with `next` commands, add `pending --stale`, and verify the `IDENTITY_REQUIRED` case
- [x] 7.4 Add the compact `attention` field to every authenticated success envelope, computed after the operation's effects in its transaction; verify the posting, nothing-waiting and last-message-consumed scenarios
- [x] 7.5 Render ages in days, hours and minutes and the attention field under `--human`, and verify with render tests
- [x] 7.6 Document `tala` as the work-boundary check, the `attention` field and background waiting in the agent guide, and verify the flow end to end without any harness integration

## 8. Integration

- [ ] 8.1 Run all evaluation scenarios for Claude Code, Codex and OpenCode without harness integration and record, per harness, missed messages and lost handles as the evidence for or against a later hooks proposal
- [x] 8.2 Measure command latency with and without ambient attention on a home with 10,000 messages and record the result
- [x] 8.3 Verify `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` and `openspec validate --all --strict` pass
- [x] 8.4 Write upgrade and rollback release notes covering protocol 3, handles and reissue, and verify the steps on a protocol 2 home

## Workflow follow-up

- Sync delta specs into `openspec/specs/` and archive the change on the feature branch, in the implementation PR.
- Close #93, #94, #95, #97 and #98 when the PR merges; close #96 now as fixed by the protocol 2 redesign.
