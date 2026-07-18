## 1. Session Rename SSE Broadcast

- [ ] 1.1 Add `SessionRenamed { id: String, old_name: String, new_name: String }` variant to `DaemonEvent` enum in `src/models.rs:119`
- [ ] 1.2 Emit `DaemonEvent::SessionRenamed` from `rename_session()` in `src/store.rs:206` after successfully renaming
- [ ] 1.3 Handle `SessionRenamed` variant in SSE streaming match arms in `src/api.rs` (lines 297, 318, 321, 359, 368, 376, 377, 408, 411, 414, 576, 587, 590, 699, 725, 738, 753) — at minimum skip/continue like `SessionCreated`/`SessionReopened`, or surface to CLI clients

## 2. Fix `tala discover` Daemon Path

- [ ] 2.1 Update `cmd_discover()` in `src/cli.rs:1924` to read daemon.json from `tala_home()`/`daemon.json` instead of `dir.join(".tala").join("daemon.json")` (lines 1940, 1970)
- [ ] 2.2 Verify daemon status probe uses correct path to determine running/stopped state

## 3. `tala recap` Clears Unread Counts

- [ ] 3.1 Add `store::write_cursor(recap.cursor.unwrap_or(0))` call at end of `cmd_recap()` in `src/cli.rs:1769` before `Ok(())`

## 4. Improve Daemon-Not-Found Error Messages

- [ ] 4.1 In `ensure_daemon_running()` (`src/cli.rs:590`): when `read_daemon_json()` fails, check if the daemon home directory exists and emit a path-specific error instead of generic "daemon failed to start within 5 seconds"

## 5. Verify

- [ ] 5.1 Build project with `cargo build` and fix any compilation errors
- [ ] 5.2 Run `cargo test` to confirm no regressions
- [ ] 5.3 Run `cargo clippy` to verify code quality
