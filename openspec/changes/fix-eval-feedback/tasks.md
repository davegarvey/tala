## 1. Add --wait flag to tala start

- [ ] 1.1 Add `wait: bool` and `timeout_secs: Option<u64>` and `json: bool` parameters to `cmd_start`
- [ ] 1.2 In `cmd_start`, after creating the session and printing the ID, if `wait` is set, call the wait endpoint with the first message's ID (if a message was sent) or `since=0` as fallback
- [ ] 1.3 Add `--wait` / `-w` and `--timeout` / `-t` clap args to the `Start` command variant
- [ ] 1.4 Update the `run()` dispatch to pass the new args to `cmd_start`
- [ ] 1.5 Handle `--json` in the wait path and exit code 2 on timeout

## 2. Fix cursor tracking in tala wait

- [ ] 2.1 In `cmd_wait`, after printing received messages (line ~1417), compute the max message ID and call `store::write_cursor(max_id)`
- [ ] 2.2 Ensure cursor is NOT updated on timeout or closed-session responses

## 3. Add --json flag to tala init

- [ ] 3.1 Add `json: bool` field to the `Init` command variant in the Commands enum
- [ ] 3.2 Add `json_output: bool` parameter to `cmd_init`
- [ ] 3.3 Branch output in `cmd_init` to produce JSON when `json_output` is true
- [ ] 3.4 Update the `run()` dispatch to pass json flag to `cmd_init`

## 4. Surface aliases in help text

- [ ] 4.1 Add `after_help` string to the `Chat` command definition noting `tala send` as alias
- [ ] 4.2 Verify `tala chat --help` shows the alias note

## 5. Auto-start daemon in tala init

- [ ] 5.1 At the end of `cmd_init`, call `ensure_daemon_running().await?` to start the daemon
- [ ] 5.2 Handle the case where daemon start fails gracefully (log to stderr, don't fail init)

## 6. Tests

- [ ] 6.1 Add e2e test for `tala start --wait`
- [ ] 6.2 Add e2e test for `tala wait` cursor update
- [ ] 6.3 Add e2e test for `tala init --json` output format
- [ ] 6.4 Verify all tests pass with `cargo test`
