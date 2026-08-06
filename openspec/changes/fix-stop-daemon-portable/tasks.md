# Fix stop-daemon-portable — tasks

## 1. Direct dependency

- [ ] 1.1 Add `libc = "0.2"` to `[dependencies]` in Cargo.toml

## 2. cmd_stop rewrite

- [ ] 2.1 In `cmd_stop()` (src/cli.rs), replace `Command::new("kill").arg(pid)` with
      `unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) }`
- [ ] 2.2 Map ret != 0: ESRCH → treat as already-gone (remove daemon.json, print
      "daemon stopped"); other errors → return contextual error with pid
- [ ] 2.3 Keep the 20×100 ms daemon.json polling loop and stale-cleanup path unchanged

## 3. Verify

- [ ] 3.1 `cargo test test_daemon_lifecycle` passes (was failing before this change)
- [ ] 3.2 Manual: `tala stop` on this container kills the daemon (disposable TALA_HOME)
- [ ] 3.3 `cargo fmt --check` and full `cargo test` clean
