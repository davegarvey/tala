## Why

The cross-project eval critic identified three P1 and three P2 issues from two autonomous agents testing tala. The most critical gaps are: no way for agents to discover each other across projects, confusingly overlapping message-watching commands, and silent active-session changes after close/reopen. These undermine tala's core value proposition of cross-project agent-to-agent messaging.

## What Changes

- **Agent discovery**: Add `tala discover` command so agents can find and connect to agents in other projects by scanning session history and agent configs across known project roots
- **Command naming clarification**: Add help text disambiguation and cross-references between `wait`/`listen`/`stream`/`whatsup` so users can understand the difference at a glance
- **Active session integrity**: `tala use` now warns when switching active session implicitly (e.g. after close/reopen), and `tala wait`/`tala send` verify the active session is still active before using it
- **`--new` renamed to `--new-session`**: The `tala wait --new` flag renamed to `--new-session` for clarity
- **`tala listen` default `--since`**: Default `since` for `tala listen` changed to "latest checkpoint" instead of 0, so it only shows new messages on first connect (opt-in to full history with `--since 0`)

## Capabilities

### New Capabilities
- `cross-project-discovery`: Mechanism for agents to discover and connect to agents in other projects

### Modified Capabilities
- `cli-ux`: Message-watching help text disambiguation; `--new` renamed to `--new-session`; `listen` default behavior changed from full-history to new-messages-only
- `active-session-integrity`: Active session verification and warning on implicit switches
- `command-organization`: Top-level vs subcommand split clarification

## Impact

- `src/cli.rs` — new `discover` command, flag rename, help text updates
- `src/api.rs` — new discovery endpoint if needed, listen since default change
- `src/models.rs` — new discovery models
- `src/store.rs` — active session integrity handling
- `tests/e2e.rs` — tests for all new behaviors
