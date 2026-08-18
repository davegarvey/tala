## 1. Extend Init Controls

- [x] 1.1 Add `--dry-run`, `--force`, `--gitignore`, and `--json` options to `tala init` without changing the positional agent-name behavior.
- [x] 1.2 Define stable init action/status types and JSON serialization for config, integration files, Git-ignore setup, warnings, and dry-run state.
- [x] 1.3 Update init help and user-facing documentation to describe repeat-run safety, explicit force refresh, dry runs, JSON output, and opt-in Git-ignore setup.

## 2. Plan And Apply Safe Initialization

- [x] 2.1 Refactor init to render current integration documents and build a complete action plan before writing any project file.
- [x] 2.2 Preserve existing config identity and classify integration files as missing, identical, or different without modifying different files by default.
- [x] 2.3 Apply only planned writes, use atomic per-file replacement, and require `--force` for different existing integration files.
- [x] 2.4 Implement dry-run behavior that reports the plan without creating directories or modifying config, integration files, or Git-ignore files.
- [x] 2.5 Implement repository-root Git-ignore detection and opt-in `/.tala/` insertion with duplicate prevention and outside-repository warnings.
- [x] 2.6 Keep human diagnostics on stderr where appropriate and emit the documented JSON report on stdout when `--json` is used.

## 3. Test Init Safety

- [x] 3.1 Add help and argument parsing coverage for all init options and invalid flag combinations.
- [x] 3.2 Add E2E coverage for first init, repeat init with identical files, repeat init with locally changed files, and explicit `--force` replacement.
- [x] 3.3 Add E2E coverage proving `--dry-run` leaves all files unchanged and reports planned actions.
- [x] 3.4 Add JSON output tests covering config, integration, warning, and dry-run fields.
- [x] 3.5 Add Git-ignore tests for opt-in insertion, existing-rule idempotence, missing `.gitignore`, nested repository roots, and non-Git directories.
- [x] 3.6 Verify `--force` never changes an existing `.tala/config.json` and that missing `.opencode/` still skips integration files.

## 4. Validate And Prepare Release

- [x] 4.1 Run `cargo fmt --check`, `cargo test`, and clippy with warnings denied.
- [x] 4.2 Run `openspec validate --all` and confirm the change is ready to sync and archive.
