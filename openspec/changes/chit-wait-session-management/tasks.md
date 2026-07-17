## 1. Daemon: Wait-all endpoint

- [x] 1.1 Add `/api/sessions/wait-all` route to router
- [x] 1.2 Implement `wait_all` handler subscribing to `global_tx`
- [x] 1.3 Return `WaitResponse` with the next message from any session
- [x] 1.4 Handle timeout returning `timeout: true` response

## 2. CLI: `chit start` cleanup

- [x] 2.1 Remove `write_active_session()` call from `cmd_start`
- [x] 2.2 Remove redundant message send in `cmd_start` (duplicate fix)
- [x] 2.3 Update test `test_auto_target_single_session` to use `chit use`
- [x] 2.4 Update test `test_multiple_sessions_auto_target_sends_to_active` to use `chit use`

## 3. CLI: `chit wait` multi-session resolution

- [x] 3.1 Move `wait_timeout` computation before session resolution block
- [x] 3.2 Replace `resolve_session_id` with inline logic: explicit → active → daemon query
- [x] 3.3 Implement 0-sessions branch: print status, call wait-new, print new session ID
- [x] 3.4 Implement 1-session branch: print status, use the single session
- [x] 3.5 Implement 2+-sessions branch: print status, call wait-all, display result
- [x] 3.6 Set active session when messages arrive via wait or wait-all

## 4. CLI: `chit send` auto-create notification

- [x] 4.1 Change `eprintln!("Created session {}")` to `println!("→ Created session {}")`
- [x] 4.2 Guard with `!json_output` to avoid breaking JSON output

## 5. Eval framework

- [x] 5.1 Fix daemon lifecycle with `nohup + disown` in both setup functions
- [x] 5.2 Add `CHIT_HOME` export instructions to agent task templates
- [x] 5.3 Switch feedback from file-writing to inline Task results
- [x] 5.4 Replace `chit rename` with `chit session rename` in command reference
- [x] 5.5 Rewrite task templates with personas and exploration goals

## 6. Documentation

- [x] 6.1 Update SKILL.md with eval lessons from rounds 2-3
- [x] 6.2 Remove auto-active gotcha from SKILL.md
- [x] 6.3 Add inline feedback, wait-all, and session rename lessons
