## 1. Fix `--file -` stdin bug

- [ ] 1.1 In `cmd_send`, when `--file` value is `"-"`, set `use_stdin = true` and skip file reading, falling through to the existing stdin reading logic
- [ ] 1.2 Verify the fix handles piped input correctly with the existing timeout logic

## 2. Fix `tala discover` daemon status detection

- [ ] 2.1 In `cmd_discover`, after attempting `/api/agents` probe, add TCP port connection test as fallback
- [ ] 2.2 If port is open and accepting connections, set `daemon_running = true`
- [ ] 2.3 Verify discover shows "running" for active daemons even when `/api/agents` fails

## 3. Fix `tala agents` to show agents before messaging

- [ ] 3.1 In the `/api/agents` handler, include the daemon's own agent name from config
- [ ] 3.2 Also include agents from session participants (not just message senders) — track participants in session metadata
- [ ] 3.3 Verify agents from sibling projects appear in `tala agents` output without prior messaging

## 4. Fix self-message exclusion from unread counters

- [ ] 4.1 Read local agent name from `.tala/config.json`
- [ ] 4.2 In `compute_session_unread`, filter out messages where `sender == local_agent_name`
- [ ] 4.3 Verify own messages don't increment unread count in `tala list` and `tala status`

## 5. Add `--cursor` alias on recap

- [ ] 5.1 Add `--cursor` as a clap argument alias for `--since` in the Recap command definition
- [ ] 5.2 Verify `tala recap --cursor N` works identically to `tala recap --since N`

## 6. Enhance `tala use` output

- [ ] 6.1 Fetch session details (name, message count) when displaying active session in `cmd_use`
- [ ] 6.2 Include session name and message count in both human and JSON output
