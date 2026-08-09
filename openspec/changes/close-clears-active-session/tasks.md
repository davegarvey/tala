# Close-active-session clears active marker (B028)

## 1. Implementation

- [x] 1.1 In `cmd_close` (src/cli.rs), change `was_active` to be computed from the resolved
      session id only: `store::read_active_session().await.as_deref() == Some(&session_id)`
      (drop the `session_arg.is_none()` condition).
- [x] 1.2 Verify existing behavior preserved: `close` with no arg on the active session still
      prints "Active session was closed and cleared"; `close` on a non-active session prints
      only the close confirmation; `close --json` includes `active_cleared` only when active.

## 2. Tests (tests/e2e.rs)

- [x] 2.1 `test_session_close_alias_clears_active` — `use <sess>` then `session close <sess>`;
      assert `use` (no arg) reports no active session and `list` shows no `*` on the closed row.
- [x] 2.2 `test_close_explicit_id_clears_active` — `use <sess>` then `close <sess>`; assert the
      same clean state.
- [x] 2.3 `test_close_non_active_keeps_active_marker` — two sessions, active = A; `close B`;
      assert active still A and `*` remains on A.
- [x] 2.4 Existing tests unchanged and passing (test_close_session, test_close_json_output,
      test_send_to_closed_session_fails, test_session_close_alias).

## 3. Quality gates

- [x] 3.1 `cargo fmt --check`
- [x] 3.2 `cargo clippy -- -D warnings`
- [x] 3.3 `cargo test` (full suite)
- [x] 3.4 Live verification on shared daemon (repro from cycle-08 feedback)
