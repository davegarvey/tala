# Fix list active-column clarity — tasks

## 1. `tala list` text status column (src/cli.rs, cmd_list)

- [ ] 1.1 Change status word: `if s.closed { "closed" } else { "open" }` (was
      `"active"`); active session stays identified by the `*` marker only
- [ ] 1.2 `session list` shares `cmd_list` — verify it renders the same way

## 2. Overloaded "active" wording in resolve_session_id (src/cli.rs)

- [ ] 2.1 "Multiple active sessions: …" → "Multiple open sessions: …" (B022)

## 3. Empty-session history note (src/cli.rs, cmd_recap)

- [ ] 3.1 In the text path, after the header, if `recap.messages.is_empty()`
      print "(no messages yet)" instead of a bare blank line (B010)

## 4. Reopen must not change the active session (src/cli.rs, cmd_session_reopen)

- [ ] 4.1 Remove `store::write_active_session(&session_id)` (B012)
- [ ] 4.2 Update output: "Session X reopened (use `tala use X` to make it active)";
      JSON path: drop the `out["active"] = true` line (session is not active)

## 5. Tests first (tests/e2e.rs)

- [ ] 5.1 list text shows `open` for a non-closed session and `*` on the active
      one; never shows the word `active` in the status column
- [ ] 5.2 `history` on an empty session (or `--since` far ahead) prints
      "(no messages yet)"
- [ ] 5.3 close+reopen does NOT move the active session: set active A, close B,
      reopen B → `tala use` still reports A
- [ ] 5.4 existing `test_session_reopen*` tests still pass unchanged

## 6. Verification

- [ ] 6.1 `cargo fmt --check`
- [ ] 6.2 `cargo clippy --all-targets -- -D warnings`
- [ ] 6.3 `cargo test` (CARGO_HOME=/workspace/.cargo-home)
- [ ] 6.4 manual: `tala list` on the shared daemon shows `open`/`closed` + `*`
