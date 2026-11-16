# Proposal

## Why

An agent learns about Tala messages only when it runs a `tala` command or keeps a background `tala wait` running. An agent that spends a long task editing files and running tests does neither, so requests, supersessions and deadline alerts wait unseen until the task ends or a person relays them. That defeats Tala's purpose. Every evaluation so far ran sessions of 35–90 seconds in which messaging was the task, and each report lists long sessions and context compaction as untested, so the evidence behind the current hookless design cannot detect this failure.

The harnesses that matter most to Tala's users in practice now offer the hooks needed to close the gap: Claude Code (CLI and the desktop app's Code tab) and Codex (CLI and desktop app) run command hooks after each tool call and at session start, including after compaction, and OpenCode runs plugin files on tool and session events. Each uses a different mechanism and configuration file, so harness support must not leak into the core CLI.

## What Changes

- Add a `tala connector` command group (`list`, `install`, `remove`, `check`, `hook`) that installs, inspects and removes opt-in harness connectors, and runs their hooks.
- Add a connector catalogue bundled in the binary. Each connector is a declarative manifest plus template files describing the harness events it uses, the configuration it writes and how Tala's attention is rendered for that harness. Adding a harness means adding a connector directory, not changing the messaging core.
- Ship three connectors: `claude-code`, `codex` and `opencode`.
- Add a harness-neutral hook runner (`tala connector hook`) that maps a harness session to a private per-session selector file, stays silent when the session has not registered or nothing has changed, and otherwise reports the attention summary in the connector's output shape.
- At session start, and again after compaction, connectors remind a registered agent of its selector file, so a compacted agent no longer loses its identity.
- Replace the `agent-cli` rule that Tala never changes harness configuration with a narrower one: only an explicit `tala connector install` or `remove` may change it, and `init` still never does.
- Add a long-session evaluation scenario that measures missed and late requests with and without a connector.

No existing command, output field or wire message changes. `PROTOCOL_VERSION` is unchanged.

## Capabilities

### New Capabilities
- `harness-connectors`: the connector catalogue and manifest format, installation and removal of harness configuration, the session-keyed hook runner, change-only attention notifications and post-compaction identity reminders.

### Modified Capabilities
- `agent-cli`: the command surface gains the `connector` group; the agent-integration requirement permits harness configuration changes through explicit connector commands only.

## Impact

- Code: new `src/connector.rs` (catalogue, install planner, hook runner); `src/cli.rs` (the `connector` group); new `connectors/<name>/` directories embedded with `include_str!`. The hook runner reuses the existing attention summary and selector-file handling.
- Files written outside the repository, only on explicit install: `~/.claude/settings.json` or `.claude/settings.local.json`; `~/.codex/hooks.json` or `.codex/hooks.json`; `~/.config/opencode/plugins/tala.ts` or `.opencode/plugins/tala.ts`. Per-session selector and notification state under `$TALA_HOME/sessions/`.
- Users must still approve hooks where a harness requires it (Codex desktop: Settings → Hooks; Codex CLI: `/hooks`).
- Docs: README, `docs/agent-guide.md` (connectors in the existing `harness` topic) and `eval/scenarios/long-session.md`.
- Dependencies: none expected. JSON configuration is merged with `serde_json`; TOML is not written.
