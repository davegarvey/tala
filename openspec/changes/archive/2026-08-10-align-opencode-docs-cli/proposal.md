# Align .opencode skill/command docs with the real CLI surface

## Why

The agent-facing docs shipped with the repo (`.opencode/skills/tala/SKILL.md` and
`.opencode/commands/tala.md`) document commands that do not exist in the binary:
`tala start`, `tala recap`, `tala wait --new`, `tala use <name>`-only, `tala init <name>`,
and a `-s/--session` flag on `recap`/`close`/`follow` (the `follow` command was removed).
An agent onboarding through the skill — the primary usage path — hits
`error: unrecognized subcommand 'start'` on its first command. Cycle-01 and cycle-02
feedback both flagged this (B001). README.md is accurate; the embedded template inside
`install_opencode_skills()` (src/cli.rs) is partially stale too: it claims
"If no session exists and you provide a message, auto-creates a session", which is false
(verified: `tala send "msg"` with no active session errors and lists sessions).

## What Changes

- Rewrite `.opencode/skills/tala/SKILL.md` to the real command surface: `init`, `use`,
  `send`, `wait`, `stream`, `history`, `listen`, `check`, `list`, `discover`, `agents`,
  `close`, `status`, `stop`, `session` (create/show/rename/reopen/close). Document
  verified behaviors: `use` name/prefix matching with ambiguity errors, `history`
  `--since/--from/--limit`, `wait --new-session` semantics (waits for a session created
  after the command starts), no auto-create on `send`.
- Rewrite `.opencode/commands/tala.md` to match (single-paragraph summary).
- `install_opencode_skills()` (src/cli.rs) loads both files via `include_str!("../.opencode/...")`
  so the binary ships exactly the repo docs — one source of truth, no embedded drift.
- New e2e test: every `tala <command>` referenced in backtick/code-fence spans of
  README.md + `.opencode/` docs must exist in `tala --help`.

## Capabilities

### New Capabilities
- `doc-consistency-check`: e2e test that validates doc-referenced commands against the
  built binary's help output.

### Modified Capabilities
- `agent-onboarding`: skill/command docs now describe the actual CLI; agents following
  them no longer hit unrecognized-subcommand errors on the first command.
- `skill-install`: `tala init` installs docs identical to the repo's committed files.

## Impact

- `.opencode/skills/tala/SKILL.md` — full rewrite
- `.opencode/commands/tala.md` — full rewrite
- `src/cli.rs` — `install_opencode_skills()`: use `include_str!` for both files
- `tests/e2e.rs` — new doc-consistency test + init-installs-repo-docs test
- `openspec/changes/align-opencode-docs-cli/tasks.md` — task list

## Backlog references

- B001 (doc-vs-binary drift, corrected target), M5 (use-by-name now documented),
  B009 (3-command happy path now honestly documented)
