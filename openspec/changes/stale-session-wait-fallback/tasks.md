## 1. Wait Fallback

- [ ] 1.1 Restructure `cmd_wait` with a retry loop — wrap the session resolution + wait request in a `loop { ... }`. The session resolution block (lines 905-1004) stays inline. The wait-all branch terminates via `return`, exiting the loop.
- [ ] 1.2 Add stale session recovery — before consuming the response body, check `resp.status()`. If not successful and error contains "session not found" and session was not from explicit `--session` arg, call `store::clear_active_session().await?` then `continue` the loop
- [ ] 1.3 Move spinner creation into the loop body so it's re-created on recovery
- [ ] 1.4 Ensure the recovery path preserves all flags: `--since`, `--limit`, `--from`, `--json`, `--timeout`

## 2. Tests

- [ ] 2.1 Add e2e test for stale active session with one active session — verify fallback works and messages are received
- [ ] 2.2 Add e2e test for stale active session with no active sessions — verify it waits for new session
- [ ] 2.3 Add e2e test for stale active session with multiple active sessions — verify wait-all behavior
- [ ] 2.4 Add e2e test for stale session with `--json` flag — verify JSON output is valid (recovery messages go to stderr)
