## Why

Tala's agent instructions are copied into each repository, while the CLI is upgraded independently. A newer CLI may intentionally remove commands, leaving an agent to follow repository-local instructions that no longer match the installed binary. Tala needs a lightweight, non-blocking compatibility check so stale or unversioned integrations are visible at the moment they are used.

## What Changes

- Have normal Tala invocations inspect the nearest repository-local Tala integration when one is present.
- Compare integration metadata with the installed CLI and warn on stale, missing, or unversioned documents without blocking the requested command.
- Keep the installed binary and its public command surface authoritative; do not restore removed commands or add compatibility aliases solely for stale documentation.
- Make the warning actionable by directing agents to `tala --help` and an explicit integration refresh/check flow.
- Ensure unknown-command failures can point at potentially stale project instructions when a removed command is invoked.
- Preserve machine-readable command output by sending compatibility notices to stderr.

## Capabilities

### New Capabilities

### Modified Capabilities

- `skill-cli-compatibility`: Extend version metadata guidance with CLI-side detection and actionable warnings for stale, missing, or unversioned project integrations.
- `project-setup`: Define safe checking and explicit refreshing of repository-local Tala skill and command files without changing project identity configuration.
- `cli`: Define the intentionally reduced public command surface so generated integration documents and the installed binary have one authoritative set of commands.
- `agent-discovery`: Retain cross-project discovery while removing the local `tala agents` command from the public surface.
- `daemon-compat`: Remove the retired `tala agents` command from the read-only compatibility exception list.
- `message-parts`: Remove the retired `stream` command from the public message-surface contract.

## Impact

- CLI startup and error handling, including project-local integration discovery and compatibility diagnostics.
- `tala init` integration installation and refresh behavior.
- Embedded OpenCode skill and command templates, their metadata, and related documentation tests.
- No daemon wire-protocol, persistence, or external dependency changes are expected.
