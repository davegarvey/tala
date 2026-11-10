# Proposal

## Why

All commands sit at the top level, which mixes the messaging verbs agents run constantly with occasional set-up commands. Every new area adds more top-level names (`list-projects`, `close-project`, `reopen-project`, and the planned network commands). Output selection also still carries the `--human` and `--json` aliases from before `--format` existed. Compatibility layers like these are technical debt in a tool whose consumers re-read generated instructions.

## What Changes

- **BREAKING:** `list-projects`, `close-project` and `reopen-project` become `tala project list|close|reopen`. The old names are removed.
- **BREAKING:** `--human` and `--json` are removed; `--format=json|text` is the only output selector, defaulting to `json`.
- AGENTS.md records the command-structure rule: flat verbs for agent messaging, `noun verb` groups for occasional set-up and administration, `status` and `stop` top-level, and breaking CLI changes released as a major version.

## Capabilities

### Modified Capabilities

- `agent-cli`: command surface, input/output contract and project discovery.

## Impact

`src/cli.rs`, `src/main.rs`, `src/models.rs` help text, README, agent guide, generated OpenCode integration, AGENTS.md and end-to-end tests. Harness scripts that pass `--human` or `--json`, and instructions that name the old project commands, must be updated. The release is a major version.
