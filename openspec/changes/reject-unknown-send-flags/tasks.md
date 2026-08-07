# Reject unknown flags in `tala send` — tasks

## 1. Tests first (tests/e2e.rs)

- [ ] 1.1 e2e: `tala send --new "msg"` with an active session → exit 2, clap usage
      error mentioning `--new`, and NO message added to the active session
      (history shows no new message)
- [ ] 1.2 e2e: `tala send --timeot 1` with an active session → exit 2, NO message
      content "1" delivered
- [ ] 1.3 e2e: `tala send -- --dashed-content` still sends the message (exit 0,
      content `--dashed-content`)
- [ ] 1.4 e2e: `tala send -s <sess> "msg"` explicit session flag still works (exit 0)
- [ ] 1.5 e2e: typo'd flag with a space-separated value (`--timeot 1`) does not emit a
      cryptic partial error — clean clap usage error, message unsent

## 2. Implementation (src/cli.rs)

- [ ] 2.1 Remove `allow_hyphen_values` from the SESSION positional of `send`; keep the
      MESSAGE positional's hyphen tolerance where needed; verify `--` separator path
- [ ] 2.2 Fix the leading-`--` warning (B015): only warn when the message actually
      begins with `--` and was not separated by the explicit `--` marker

## 3. Docs

- [ ] 3.1 Align `.opencode/skills/tala/SKILL.md` + `.opencode/commands/tala.md`
      wording on dashed content: use `--` separator / `--stdin` / piping

## 4. Verify

- [ ] 4.1 `cargo fmt --check`, `cargo clippy -- -D warnings`, full `cargo test`
- [ ] 4.2 Manual spot-check on shared daemon: unknown flag errors, `--` separator works
