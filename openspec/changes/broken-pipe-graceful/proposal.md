# Die quietly on broken pipe (B038)

## Why

Backlog B038 (new, cycle-17): `tala` **panics** when its stdout pipe closes
early. Repro (3/3 live): `tala history <sess with 750 msgs> --json | head -c
40` → exit 101, stderr spew:

```
thread 'main' panicked at library/std/src/io/stdio.rs:1165:9:
failed printing to stdout: Broken pipe (os error 32)
```

First observed racier: `tala list | head -6` right after daemon auto-start.
`tala <cmd> | head`, `| grep -m1`, `| jq` are bread-and-butter agent
pipelines; a panic trace dumped into them is garbage an agent will mis-parse.
Root cause: Rust's runtime ignores SIGPIPE by default, so `println!` panics
on EPIPE instead of letting the OS terminate the process.

## What Changes

One line in `main()` plus a small helper:

- **`src/main.rs`**: `#[cfg(unix)] fn reset_sigpipe()` — sets SIGPIPE back to
  `SIG_DFL` before the CLI runs, so a closed pipe kills the process quietly
  (standard Unix-tool behavior: `grep`, `git`, `ls` all do this) instead of
  printing a panic.

## Capabilities

- `tala <cmd> | head -c N` / `| grep -m1` with an early-closing reader exits
  silently (SIGPIPE death, no panic spew, no partial-JSON corruption noise).
- All other behavior unchanged; the daemon never inherits the change in a
  harmful way (it writes no stdout).

## Impact

- Unix-only (`#[cfg(unix)]`); other platforms keep current behavior.
- Exit code on SIGPIPE death is 141 (128+SIGPIPE) as seen by a shell, or a
  signal-death status in `wait()` — standard and script-detectable.
- No API/daemon changes. No dependency on unmerged PRs — clean base vs main
  caf8718.
