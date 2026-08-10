# Fix wait-new-session handshake race — tasks

## 1. API: pre-existing-session scan + sender filter (src/api.rs)

- [x] 1.1 Add `sender: Option<String>` to `WaitNewParams`
- [x] 1.2 On entry, when `sender` is present: scan `list_sessions()` for a session
      with ≥1 message from a different sender; return `{session_id, message}` for
      the most-recently-active match
- [x] 1.3 In the event loop, when `sender` is present: `SessionCreated` no longer
      satisfies the wait; `NewMessage` satisfies it only when
      `msg.sender != sender`
- [x] 1.4 When `sender` is absent, keep legacy behavior exactly

## 2. CLI: pass caller identity (src/cli.rs)

- [x] 2.1 `cmd_wait_new`: resolve local agent name (`read_project_config()` or
      default sender) and pass `sender` query param
- [x] 2.2 `cmd_wait` no-active-session fallback: pass the same `sender` param

## 3. Tests (tests/e2e.rs)

- [x] 3.1 Test: alpha creates session + message BEFORE beta's `wait --new-session`
      starts → wait returns alpha's session id + message (pre-existing scan)
- [x] 3.2 Test: beta's own `session create` while waiting does NOT self-trigger;
      a later session+message from alpha does

## 4. Verify

- [x] 4.1 `cargo fmt --check` clean
- [x] 4.2 Full `cargo test` passes
- [x] 4.3 Manual two-dir sandbox repro: alpha's pre-existing question is delivered
      to beta's `wait --new-session`; beta's own create is ignored
