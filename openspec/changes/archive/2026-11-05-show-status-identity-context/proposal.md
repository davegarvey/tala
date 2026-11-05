# Proposal

## Why

`tala status` can report daemon and storage health but not which local selector was chosen or which registered project that identity represents. That makes identity and board-routing problems harder to diagnose, especially when the daemon is stopped or its metadata is stale.

## What Changes

- Report the selector source without printing selector values, selector-file paths, or credentials.
- When the daemon is available, include the selected public agent ID, display name, registered project root, and that project's latest board message ID.
- Define the board ID as a read-only high-water mark, not a receipt or stored resume position.
- Report why identity context is unavailable when there is no identity, the selector is invalid, or daemon context cannot be read.
- Render the same context in JSON and human-readable status output while preserving the no-daemon-start behavior.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-cli`: Define the selector source, identity/project fields, and JSON/human output for status.
- `durable-daemon`: Define context availability while preserving status as a no-start diagnostic.

## Impact

`src/cli.rs` will resolve status selector provenance without requiring identity for global daemon diagnostics, and will render status context. `src/client.rs` will support authenticated requests against an already-running daemon only. The existing `Whoami` and `Board` RPCs provide context; no wire operation or protocol version change is needed.
