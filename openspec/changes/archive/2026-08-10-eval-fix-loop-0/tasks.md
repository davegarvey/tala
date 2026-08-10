## 1. Cross-project discovery — tala discover

- [x] 1.1 Add `Discover` variant to Commands enum with `--json` flag
- [x] 1.2 Implement `cmd_discover()` — scan parent dirs up to 3 levels for `.tala/config.json`
- [x] 1.3 For each found config, read agent name and try daemon.json + `/api/agents` query
- [x] 1.4 Scan sibling directories under each parent for additional `.tala/config.json`
- [x] 1.5 Format output as table (human) or JSON array (`--json`)
- [x] 1.6 Add dispatch in `run()` for `Commands::Discover`
- [x] 1.7 Update embedded SKILL.md (tala init template) to reference `tala discover`

## 2. Help text cross-references for message-watching commands

- [x] 2.1 Add `after_help` to `Wait` listing stream/listen/whatsup/recap
- [x] 2.2 Add `after_help` to `Listen` listing wait/stream/whatsup
- [x] 2.3 Add `after_help` to `Stream` listing listen/wait/whatsup
- [x] 2.4 Add `after_help` to `WhatsUp` listing wait/listen/stream/recap
- [x] 2.5 Add `after_help` to `Recap`
- [x] 2.6 Add `after_help` to `Agents` mentioning `tala discover`
- [x] 2.7 Update `Agents` empty output to mention `tala discover`

## 3. Rename --new to --new-session

- [x] 3.1 Rename `r#new` field in `Wait` from `long = "new"` to `long = "new-session"`, add `alias = "new"`
- [x] 3.2 Update SKILL.md references from `--new` to `--new-session` (keep `--new` as alias)
- [x] 3.3 Update README.md if it references `--new`

## 4. Listen default since from cursor

- [x] 4.1 In `cmd_listen`, change default `since` from `0` to `read_cursor().unwrap_or(0)`
- [x] 4.2 After receiving each message in listen loop, update cursor via `store::write_cursor(msg.id)`
- [x] 4.3 Ensure `--since` flag still overrides the cursor-based default

## 5. Active session integrity on close/reopen

- [x] 5.1 In `cmd_close`, after successful close when session came from implicit active session, clear active session and print warning
- [x] 5.2 Add `clear_active_session` call on close when active session id matches closed session and no explicit arg was given
- [x] 5.3 In `cmd_session_reopen`, after successful reopen, write session as active via `store::write_active_session`
- [x] 5.4 Update reopen output message to indicate session is now active

## 6. Command organization hints

- [x] 6.1 Add `after_help` to `Use` mentioning `tala session`
- [x] 6.2 Add `after_help` to `SessionCommands::List` mentioning top-level `tala list`
- [x] 6.3 Add `after_help` to `SessionCommands::Close` mentioning top-level `tala close`

## 7. Tests

- [x] 7.1 Add e2e test for active session cleared on close
- [x] 7.2 Add e2e test for reopen sets active session
- [x] 7.3 Add e2e test for --new-session alias backward compat (--new still works)
- [x] 7.4 Add e2e test for listen default since from cursor
- [x] 7.5 Verify all tests pass with `cargo test`
