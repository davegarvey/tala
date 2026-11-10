# Tasks

- [x] 1.1 Add `--format=json|text` as a global option; resolve `--human` to text, accept `--json` without effect, hide both aliases, and reject `--human` with an explicit `--format`.
- [x] 1.2 Render usage errors as text when `--format=text`, `--format text` or `--human` precedes `--`.
- [x] 1.3 Update help text, README and the agent guide; verify help lists `--format` and not `--human`.
- [x] 1.4 Add end-to-end coverage for `--format=text`, `--format=json`, the `--human` alias, the conflict and text usage errors.
- [x] 1.5 Run fmt, clippy, tests and `openspec validate --all`; sync and archive.
