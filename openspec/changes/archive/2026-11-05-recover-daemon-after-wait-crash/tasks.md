# Tasks

## 1. Async daemon startup coordination

- [x] 1.1 Replace blocking startup-lock acquisition with nonblocking async retries that recheck daemon liveness and preserve unreachable-daemon behavior.
- [x] 1.2 Ensure startup-lock release is explicit on the healthy-daemon path and every startup failure path.

## 2. Regression coverage

- [x] 2.1 Add an isolated-home end-to-end test that kills the daemon while `wait` runs and verifies a concurrent inbox command completes under a hard deadline.
- [x] 2.2 Verify no replacement daemon is started when a recorded daemon is unreachable rather than refused.

## 3. Integration verification

- [x] 3.1 Run formatting, Clippy, the complete test suite, and `openspec validate --all`.
- [x] 3.2 Sync the durable-daemon delta into the main spec and archive this change on the feature branch.
