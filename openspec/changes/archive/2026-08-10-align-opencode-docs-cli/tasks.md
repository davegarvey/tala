# Align .opencode docs with CLI — tasks

## 1. Canonical docs (repo files)

- [x] 1.1 Rewrite `.opencode/skills/tala/SKILL.md` — real command surface
      (init/use/send/wait/stream/history/listen/check/list/discover/agents/close/status/
      stop/session), verified flag tables, honest behavior notes (no auto-create;
      use name/prefix matching; wait --new-session waits for post-start sessions)
- [x] 1.2 Rewrite `.opencode/commands/tala.md` — same surface, one-paragraph summary

## 2. Single source of truth

- [x] 2.1 `install_opencode_skills()`: load both files with
      `include_str!("../.opencode/skills/tala/SKILL.md")` and
      `include_str!("../.opencode/commands/tala.md")`; drop the embedded raw strings

## 3. Tests

- [x] 3.1 New e2e test: extract `tala <cmd>` tokens from backtick spans + fenced code
      blocks of README.md and `.opencode/` docs; assert each exists in `tala --help`
- [x] 3.2 New e2e test: `tala init` in a harness dir installs SKILL.md identical to the
      repo file, containing no `tala start` / `tala recap` references

## 4. Verify

- [x] 4.1 `cargo fmt --check` and full `cargo test` clean
- [x] 4.2 Manual spot-check: `tala session create --name x` flow matches SKILL.md text
