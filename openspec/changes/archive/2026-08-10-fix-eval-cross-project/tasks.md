## 1. Investigate and fix session auto-close

- [x] 1.1 Investigate root cause: confirm daemon idle timeout (600s default) drops sessions; check for other session-close code paths
- [x] 1.2 Increase default idle timeout from 600s to 86400s in store.rs and daemon.rs
- [x] 1.3 Persist open sessions to ~/.tala/sessions.json on daemon shutdown (SIGTERM, idle timeout)
- [x] 1.4 Reload persisted sessions on daemon startup, marking them as open
- [x] 1.5 Ensure no background process marks sessions as closed without explicit `tala close`

## 2. Add delivery indication to tala start

- [x] 2.1 In `cmd_start`, after printing session ID, query daemon for active session count / agent presence
- [x] 2.2 Print delivery indication: "→ No agents currently listening" or "→ N agents listening"
- [x] 2.3 Handle JSON output mode (add `agents_listening` field to JSON response)

## 3. Rename --file to --message-file on tala send

- [x] 3.1 Rename clap arg from `file` to `message_file` with `long = "message-file"`
- [x] 3.2 Add `--file` as a hidden alias with deprecation warning
- [x] 3.3 Update help text to clarify reads message content from file
- [x] 3.4 Update `cmd_send` to accept new param name

## 4. Surface --new-session in top-level help

- [x] 4.1 Add reference to `tala wait --new-session` in the top-level `#[command]` long_about or after_help text

## 5. Tests

- [x] 5.1 Add e2e test for delivery indication on `tala start`
- [x] 5.2 Add e2e test for `--message-file` flag
- [x] 5.3 Add e2e test for `--file` deprecation warning
- [x] 5.4 Verify all tests pass with `cargo test`
