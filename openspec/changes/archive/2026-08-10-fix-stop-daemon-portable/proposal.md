# Stop the daemon without an external `kill` binary

## Why

`tala stop` is broken on any system where the external `kill` utility is not in PATH
(verified on this dev container: no `/bin/kill`, no `/usr/bin/kill`; `which kill` empty).
`cmd_stop` shells out to `Command::new("kill")`, which fails with
`failed to run kill / No such file or directory (os error 2)`, exit 1 — and the daemon
keeps running. The repo's own integration suite catches this: `test_daemon_lifecycle`
asserts `status` shows "no daemon" after `tala stop`, and it FAILS (68/69 tests pass).
`tala stop` is the only advertised way to stop the background daemon, so a silent
non-functional lifecycle command is a real ops/trust gap.

## What Changes

- `cmd_stop` sends SIGTERM directly via `libc::kill(pid, SIGTERM)` instead of spawning an
  external `kill` binary. `libc` is already in the dependency tree (transitive, 0.2.x); it
  becomes a direct dependency.
- ESRCH (process already gone) keeps the existing "clean up stale daemon.json" behavior
  that the old non-zero-exit path provided.
- The post-signal polling loop (20 × 100 ms, then stale daemon.json cleanup) is unchanged.
- No API, model, or store changes.

## Capabilities

### New Capabilities
- *(none)*

### Modified Capabilities
- `daemon-lifecycle`: `tala stop` now terminates the daemon on platforms without an
  external `kill` binary. Behavior on normal systems is unchanged (daemon stops,
  daemon.json removed, exit 0).

## Impact

- `Cargo.toml` — add `libc = "0.2"` to dependencies
- `src/cli.rs` — `cmd_stop()`: replace `Command::new("kill")` with `libc::kill`
- `tests/e2e.rs` — `test_daemon_lifecycle` goes from failing to passing (no change needed
  to the test itself; the existing assertion is the regression test)
- `openspec/changes/fix-stop-daemon-portable/tasks.md` — task list
