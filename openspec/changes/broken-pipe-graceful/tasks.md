# Die quietly on broken pipe (B038) — tasks

## 1. src/main.rs

- [ ] 1.1 Add `#[cfg(unix)] fn reset_sigpipe()`: `unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL); }`
- [ ] 1.2 Call it as the first statement in `main()` (before tracing/CLI parse)

## 2. Tests (tests/e2e.rs — write first)

- [ ] 2.1 `test_broken_pipe_no_panic`: seed a session with enough messages to
      make history output large; spawn `history <sess> --json` with piped
      stdout; drop the read end immediately (deterministic EPIPE); assert the
      child's stderr does NOT contain "panicked" and exit code is not 101.

## 3. Quality gates

- [ ] 3.1 `cargo fmt --check` clean
- [ ] 3.2 `cargo clippy -- -D warnings` clean
- [ ] 3.3 `cargo test` green (full suite)
- [ ] 3.4 Live-verify: `tala history <big> --json | head -c 40` exits silently
      (no panic), and normal commands still work.
