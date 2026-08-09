## Context

`cmd_stop()` in `src/cli.rs` currently spawns the external `kill` binary:

```rust
let kill_status = Command::new("kill")
    .arg(info.pid.to_string())
    .status()
    .context("failed to run kill")?;
```

On systems without a `kill` utility in PATH (verified on the PO dev container), the spawn
fails with ENOENT and `anyhow` reports `failed to run kill / No such file or directory`.
The daemon keeps running; `tala stop` is effectively dead. The e2e suite detects this:
`test_daemon_lifecycle` asserts `status` shows `no daemon` after `tala stop` and currently
fails. The daemon is a child process started by the CLI (it writes `daemon.json` with its
PID), so sending SIGTERM to that PID via the `kill(2)` syscall is the correct portable fix.

## Goals / Non-Goals

**Goals:**
- `tala stop` terminates the daemon without requiring an external `kill` binary.
- Preserve existing UX: "daemon is not running" when no daemon.json; "daemon stopped"
  on success; stale daemon.json cleanup when the process is already gone.

**Non-Goals:**
- No change to daemon startup, idle timeout, or daemon.json format.
- No Windows support work (`stop` remains a no-op bail on non-unix).

## Design

Add `libc = "0.2"` (already in the lock file transitively) and replace the `kill` spawn:

```rust
let pid = info.pid as libc::pid_t;
let ret = unsafe { libc::kill(pid, libc::SIGTERM) };
if ret != 0 {
    let err = std::io::Error::last_os_error();
    if err.raw_os_error() == Some(libc::ESRCH) {
        // Process already gone — clean up stale daemon.json (matches old non-zero-exit path)
        store::remove_daemon_json().await;
        println!("daemon stopped");
        return Ok(());
    }
    return Err(anyhow::anyhow!("failed to kill daemon (pid {}): {}", pid, err));
}
```

The subsequent loop (poll daemon.json removal up to 2 s, then force-clean) is unchanged.
`SIGTERM` is the same signal the shell `kill` (default TERM) delivered before; the daemon
already handles termination by removing daemon.json on shutdown.

## Alternatives considered

- **Shell out to `sh -c "kill $pid"`**: works on this container (dash builtin) but still
  depends on a shell and adds a string-formatting surface. libc is cleaner and already in
  the tree.
- **`nix` crate**: heavier dependency for one syscall; libc is sufficient.
