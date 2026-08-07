# Persist message history across daemon restarts — tasks

## 1. Store (src/store.rs)

- [ ] 1.1 Add `MessagesFile` serde struct: `{ messages: HashMap<String, Vec<Message>>, next_msg_id: HashMap<String, u64> }`; `messages_path()` → `tala_home()/messages.json`
- [ ] 1.2 `persist_messages(&messages, &next_msg_id)` — atomic tmp+rename write (mirror `persist_sessions`)
- [ ] 1.3 `load_messages() -> (HashMap<String, Vec<Message>>, HashMap<String, u64>)` — missing/corrupt → empty maps; next-id fallback per session = max id + 1 (or 1)
- [ ] 1.4 `Store::persist()` — also write messages + next_msg_id
- [ ] 1.5 `Store::load_persisted()` — also load messages + next_msg_id (keep existing sessions-only path when messages.json absent)
- [ ] 1.6 `Store::add_message()` — persist after inserting the message (and after updating next_msg_id), best-effort (ignore write errors like existing rename_session does)

## 2. Daemon (src/daemon.rs)

- [ ] 2.1 Verify idle-timeout + graceful-shutdown persist call sites already cover both files (they call `store.persist()`) — no change expected

## 3. Tests (tests/e2e.rs, write FIRST)

- [ ] 3.1 `test_daemon_restart_preserves_messages`: start daemon, create session, send N messages, `tala stop`, run a command to restart, assert `history` shows all N with original ids/order, `list` shows `N msgs`
- [ ] 3.2 `test_daemon_restart_preserves_session_names`: rename before stop; assert name survives restart (regression guard for metadata persistence)
- [ ] 3.3 `test_message_ids_resume_after_restart`: send 3, stop, restart, send again → new message id = 4 (no reuse/duplication)
- [ ] 3.4 `test_restart_without_messages_file`: fresh TALA_HOME, no messages.json → daemon starts clean (backward compat)

## 4. Verification

- [ ] 4.1 `cargo fmt --check`
- [ ] 4.2 `cargo clippy -- -D warnings`
- [ ] 4.3 `cargo test` (full suite)
- [ ] 4.4 Manual: rebuild binary, live repro of B024 on shared daemon → history survives restart
