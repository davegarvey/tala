## 1. Cross-project discovery — tala discover

- [ ] 1.1 Add `Discover` variant to Commands enum with `--json` flag
- [ ] 1.2 Implement `cmd_discover()` — scan parent dirs up to 3 levels for `.tala/config.json`
- [ ] 1.3 For each found config, read agent name and try daemon.json + `/api/agents` query
- [ ] 1.4 Scan sibling directories under each parent for additional `.tala/config.json`
- [ ] 1.5 Format output as table (human) or JSON array (`--json`)
- [ ] 1.6 Add dispatch in `run()` for `Commands::Discover`
- [ ] 1.7 Update embedded SKILL.md (tala init template) to reference `tala discover`

## 2. Help text cross-references for message-watching commands

- [ ] 2.1 Add `after_help` to `Wait` listing stream/listen/whatsup/recap
- [ ] 2.2 Add `after_help` to `Listen` listing wait/stream/whatsup
- [ ] 2.3 Add `after_help` to `Stream` listing listen/wait/whatsup
- [ ] 2.4 Add `after_help` to `WhatsUp` listing wait/listen/stream/recap
- [ ] 2.5 Add `after_help` to `Recap`
- [ ] 2.6 Add `after_help` to `Agents` mentioning `tala discover`
- [ ] 2.7 Update `Agents` empty output to mention `tala discover`

## 3. Rename --new to --new-session

- [ ] 3.1 Rename `r#new` field in `Wait` from `long = "new"` to `long = "new-session"`, add `alias = "new"`
- [ ] 3.2 Update SKILL.md references from `--new` to `--new-session` (keep `--new` as alias)
- [ ] 3.3 Update README.md if it references `--new`

## 4. Listen default since from cursor

- [ ] 4.1 In `cmd_listen`, change default `since` from `0` to `read_cursor().unwrap_or(0)`
- [ ] 4.2 After receiving each message in listen loop, update cursor via `store::write_cursor(msg.id)`
- [ ] 4.3 Ensure `--since` flag still overrides the cursor-based default

## 5. Active session integrity on close/reopen

- [ ] 5.1 In `cmd_close`, after successful close when session came from implicit active session, clear active session and print warning
- [ ] 5.2 Add `clear_active_session` call on close when active session id matches closed session and no explicit arg was given
- [ ] 5.3 In `cmd_session_reopen`, after successful reopen, write session as active via `store::write_active_session`
- [ ] 5.4 Update reopen output message to indicate session is now active

## 6. Command organization hints

- [ ] 6.1 Add `after_help` to `Use` mentioning `tala session`
- [ ] 6.2 Add `after_help` to `SessionCommands::List` mentioning top-level `tala list`
- [ ] 6.3 Add `after_help` to `SessionCommands::Close` mentioning top-level `tala close`

## 7. Tests

- [ ] 7.1 Add e2e test for active session cleared on close
- [ ] 7.2 Add e2e test for reopen sets active session
- [ ] 7.3 Add e2e test for --new-session alias backward compat (--new still works)
- [ ] 7.4 Add e2e test for listen default since from cursor
- [ ] 7.5 Verify all tests pass with `cargo test`
