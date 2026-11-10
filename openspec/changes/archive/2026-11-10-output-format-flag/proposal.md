# Proposal

## Why

Output selection uses `--human` for text and a hidden, no-op `--json`. The name `--human` describes an audience rather than a format, which is unusual: most CLIs use `--format=<name>` or `--output=<name>`. A single `--format` option is easier to discover in help, states the default explicitly and leaves room for further formats.

## What Changes

- Add the global option `--format=json|text`, defaulting to `json`. `text` selects the existing human-readable renderer, including for usage errors.
- Keep `--human` as a hidden alias for `--format=text`, and `--json` as a hidden flag that is accepted without effect, so existing harness scripts, wait loops and generated instructions keep working. `--human` conflicts with an explicit `--format`.
- Update help, README and the agent guide to use `--format=text`.

## Non-goals

- Changing the default format. Agents remain the primary consumers, so JSON stays the default.
- An environment variable for the format. An inherited `TALA_FORMAT=text` would silently break agents that parse JSON.
- Removing the aliases. That can follow in a major release.

## Capabilities

### Modified Capabilities

- `agent-cli`: the input and output contract selects rendering with `--format`.

## Impact

`src/cli.rs` (global option and resolution), `src/main.rs` (usage-error rendering before parsing), `src/models.rs` (help text), README, agent guide and end-to-end tests. No wire or storage change.
