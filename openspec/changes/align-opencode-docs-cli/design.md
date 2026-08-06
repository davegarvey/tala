## Context

Two agent-facing doc surfaces exist and both drift from the binary:

1. `.opencode/skills/tala/SKILL.md` + `.opencode/commands/tala.md` (committed to the repo) —
   document `tala start`, `tala recap`, `tala wait --new`, `tala use <name>`, `tala init <name>`,
   and a `follow` command that was removed in the cli-cmd-rename work. `tala start` and
   `tala recap` do not exist in the 0.25.0 binary (`error: unrecognized subcommand`, exit 2).
2. The embedded template in `install_opencode_skills()` (src/cli.rs) — closer to the real
   surface but still claims `tala send` auto-creates a session when none exists, which is
   false (verified: it errors and lists sessions, exit 1).

Both are written in raw strings inside the source or as standalone files, so there is no
enforcement that docs match `--help`. A typo or rename ships silently.

## Goals / Non-Goals

**Goals:**
- Repo `.opencode/` docs describe only real commands and real flags, with verified behaviors.
- The binary installs exactly the repo docs (`include_str!`), eliminating embedded drift.
- A test fails the build if any doc references an unknown `tala <command>`.

**Non-Goals:**
- No CLI behavior changes (the `use` name-matching and `history` pagination already work;
  they just get documented).
- No README changes (already accurate).

## Design

### Canonical docs
Rewrite both files against the verified `--help` output of tala 0.25.0:
- Commands: init, use, send, wait, stream, history, listen, check, list, discover, agents,
  close, status, stop, session (create|show|rename|reopen|close|list)
- Notable documented behaviors (all verified this cycle):
  - `send` does NOT auto-create; with no active session it errors, lists sessions, suggests `tala use <id>`.
  - `use` matches by name, then ID prefix, then full ID; ambiguous → "Multiple sessions match" error.
  - `history --since/--from/--limit` compose; `--limit` returns the first N (oldest) of the filtered set.
  - `wait --new-session` waits for a session created *after* the command starts (known race B003).
  - `session create` sets the active session; `session reopen` also sets it active (B012).

### Single source of truth
Replace the two raw-string templates in `install_opencode_skills()` with:

```rust
const SKILL_MD: &str = include_str!("../.opencode/skills/tala/SKILL.md");
const COMMAND_MD: &str = include_str!("../.opencode/commands/tala.md");
```

`include_str!` paths resolve relative to `src/`, so `../.opencode/...` points at the repo
root files. Build fails if the files are missing; the binary always ships the committed docs.

### Doc-consistency test
In `tests/e2e.rs`, a new test:
1. Runs `tala --help`, parses the `Commands:` block into a set.
2. Reads README.md, `.opencode/skills/tala/SKILL.md`, `.opencode/commands/tala.md` from
   `CARGO_MANIFEST_DIR`.
3. Extracts backtick spans and fenced code blocks; within them finds `tala <word>` tokens.
4. Asserts every first-word token is in the command set (allowlist: `cli` from
   `cargo binstall tala-cli`). Tokens starting with `-` are ignored (flags).

This makes the docs-to-binary contract machine-checked on every PR.

## Alternatives considered

- **Generate docs from clap at build time**: heavy; help text is prose, not a doc format.
- **Keep raw strings and add a manual checklist**: no enforcement; exactly what failed.
