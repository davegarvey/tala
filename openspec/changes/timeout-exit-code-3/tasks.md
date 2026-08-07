# Dedicated exit code 3 for benign blocking timeouts — tasks

## 1. CLI (src/cli.rs)

- [x] 1.1 `const EXIT_TIMEOUT: i32 = 3` with rationale comment
- [x] 1.2 `send --wait` timeout → exit 3 (text + JSON paths)
- [x] 1.3 `wait <sess>` timeout → exit 3 (text + JSON paths)
- [x] 1.4 `wait --new-session` timeout → exit 3 (dedicated command + cmd_wait fallback)
- [x] 1.5 Help text: document exit codes in `send` and `wait` after_help

## 2. Tests (tests/e2e.rs, write FIRST)

- [x] 2.1 `test_wait_timeout_exits_3` (text + JSON)
- [x] 2.2 `test_send_wait_timeout_exits_3`
- [x] 2.3 `test_wait_new_session_timeout_exits_3`

## 3. Verification

- [x] 3.1 `cargo fmt --check`
- [x] 3.2 `cargo clippy -- -D warnings`
- [x] 3.3 `cargo test` (full suite: 79/79 integration + 14 unit; 3 new tests pass; earlier 8 failures = known parallel daemon-spawn flakiness, green on rerun)
