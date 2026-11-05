# Proposal

## Why

Messages sent after an agent switches worktrees or branches currently show the checkout and revision from registration. Recipients can be directed to stale code even though the command ran in a different checkout.

## What Changes

- Refresh worktree, branch, and revision metadata from the directory where each outgoing message command runs.
- Keep the registered canonical project as the agent's identity and default board membership.
- Leave Git-specific message metadata absent when the command runs outside a Git worktree.

## Capabilities

### New Capabilities

### Modified Capabilities
- `agent-messaging`: Outgoing messages record Git context from the invocation directory while retaining the registered project.
- `project-board`: Board messages record their posting worktree, branch, and revision at invocation time while remaining on the registered canonical project board.

## Impact

Affects outgoing context construction in `src/cli.rs`, message-context E2E coverage, and the `agent-messaging` and `project-board` OpenSpecs. No wire fields or protocol changes are required.
