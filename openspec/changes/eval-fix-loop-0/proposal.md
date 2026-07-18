## Why

The cross-project eval revealed four P1 UX issues that erode user trust in tala's core CLI: silent failures on `tala watch`, invisible progress during `tala send --wait`, naming ambiguity between wait/watch/listen/send--wait, and a deprecated command still misleading users. These are the highest-ROI fixes in an otherwise solid tool.

## What Changes

- Add progress heartbeat to `tala send --wait` so users see activity during the 300s timeout
- Add non-empty output to `tala watch` when no messages match (print a notice or empty JSON array)
- Rename `tala watch` to `tala stream` and deprecate `watch` as a hidden alias
- Bump `tala observe` deprecation to an error-level warning with a more prominent message
- Change `tala listen` description to clarify it covers all sessions (vs stream for one session)
- Improve `tala session rename` error message to hint at `--force`
- Change `tala send` to default to `--wait` when no explicit message source is given and stdin is a TTY

## Capabilities

### New Capabilities
- `cli-ergonomics`: Naming clarity, help text improvements, and deprecation handling for the `wait`/`watch`/`listen`/`send` family

### Modified Capabilities
- (none — this change does not modify existing spec-level requirements; the behavioral changes are additive)

## Impact

- `src/cli.rs`: Command definitions, help text, handler logic for send --wait heartbeat, watch empty output, and wait/watch/listen naming
- `tests/e2e.rs`: New tests for heartbeat output, watch empty output, and renamed commands
