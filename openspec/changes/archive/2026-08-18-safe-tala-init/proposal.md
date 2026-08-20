## Why

`tala init` is safe for an existing `.tala/config.json`, but it currently
rewrites project-local agent integration files every time it runs. That makes a
routine upgrade or repeated initialization capable of destroying local edits,
and it gives agents no preview or structured report of the changes. Project
ignore rules are also currently implicit rather than an explicit user choice.

## What Changes

- Keep `tala init` as the single first-run and repeat-run setup command.
- Make repeated initialization non-destructive by default: create missing
  files, skip identical files, and warn before leaving changed files untouched.
- Add explicit `--dry-run`, `--force`, `--gitignore`, and `--json` controls for
  previews, intentional replacement, ignore-rule setup, and automation.
- Require `--force` for overwriting changed generated integration files.
- Add opt-in Git-ignore setup for the project-local `/.tala/` state directory;
  never edit `.gitignore` implicitly.
- Report planned and completed actions, warnings, skipped files, and ignore-rule
  decisions in human and JSON output.
- Preserve existing project identity and OpenCode auto-detection behavior.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `openspec/specs/project-setup/spec.md`: Make repeated initialization safe,
  add explicit initialization controls, and define opt-in Git-ignore behavior.

## Impact

- Affected source: CLI init argument parsing, initialization planning, generated
  file writes, and Git repository detection.
- Affected tests: initialization E2E coverage and structured output tests.
- Affected documentation: CLI help and README initialization guidance.
- No daemon, wire-protocol, or runtime dependency changes are required.
