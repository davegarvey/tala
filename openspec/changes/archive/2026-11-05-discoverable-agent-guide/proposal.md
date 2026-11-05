# Proposal

## Why

New agents can discover Tala through `--help`, but the detailed bundled agent guide is only written into a project by `tala init`. Agents that begin with the CLI have no direct way to read the guide or locate the background wait workflow.

## What Changes

- Add a local `tala guide [topic]` command that prints the bundled guide as JSON by default and Markdown with `--human`.
- Expose the five documented topics: listening, requests, identity, worktrees, and errors.
- Point CLI help and successful registration output to `tala guide listening`.
- Align the README and bundled guide around the unified `tala wait` workflow, including how to fetch an event and restart the wait.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-cli`: add the guide command and its output behavior, and point first-run users to the guide.

## Impact

Changes the CLI command surface, bundled documentation, README, and CLI tests. The guide command is local and does not require an identity or daemon. Optional skill installation is outside this change.
