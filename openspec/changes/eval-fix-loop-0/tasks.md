## 1. Session Reopen Fix

- [ ] 1.1 Fix reopen_session in api.rs to consistently set closed=false and verify send_message allows sending after reopen

## 2. TALA_HOME Error Messages

- [ ] 2.1 Fix daemon resolution error messages to check $TALA_HOME first before falling back to ~/.tala/

## 3. Listen Default Timeout

- [ ] 3.1 Add default 300s timeout to `tala listen` command
- [ ] 3.2 Check user config for default_timeout before falling back to 300

## 4. Session Rename Persistence

- [ ] 4.1 Add sessions.json persistence in store.rs (write on rename, load on daemon start)
- [ ] 4.2 Ensure session name is not overwritten by counterparty messages

## 5. Daemon Stop Message

- [ ] 5.1 Improve `tala stop` to print "daemon is not running" when daemon.json doesn't exist

## 6. CLI Polish

- [ ] 6.1 Add `tala session close` subcommand
- [ ] 6.2 Clarify help text for `tala listen` vs `tala stream`
- [ ] 6.3 Improve `--file` flag discoverability in help text
- [ ] 6.4 Improve `tala init` help text
