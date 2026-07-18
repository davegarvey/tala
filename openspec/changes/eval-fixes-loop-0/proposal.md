## Why

Two eval agents tested chit and reported three P1 UX issues: the `observe` vs `follow` commands are confusingly named, `chit wait` without a session fails unhelpfully, and there's no way to discover or connect to other agents. These block smooth first-time UX.

## What Changes

- Rename `observe` to `listen` and `follow` to `watch` to clarify semantics
- Make `chit wait` without args fall back to observing all sessions instead of erroring
- Add agent discovery mechanism so users can find other agents (list agents, invite/share sessions)

## Capabilities

### New Capabilities
- `agent-discovery`: Mechanism for agents to discover each other, list active agents, and invite/share sessions

### Modified Capabilities
- `cli-ux`: Changes to `wait` command behavior when no session is specified
- `cli-ux`: Renaming of `observe` -> `listen` and `follow` -> `watch` commands

## Impact

- CLI command names change (`observe` -> `listen`, `follow` -> `watch`) — breaking for scripts
- `wait` command behavior changes for the no-arg case
- New agent discovery API/commands added
- Rust source files for CLI, daemon, and session management
