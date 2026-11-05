# Proposal

## Why

Two failure paths can leave Tala unavailable or an agent unusable: replacing an existing file with `std::fs::rename` fails on Windows, and a registration committed by the daemon can be stranded if the client cannot save its private handle. These failures are recoverable with cross-platform replacement semantics and a private credential fallback.

## What Changes

- Replace staged files consistently on Windows and Unix, including daemon metadata, generated instruction refreshes, and rollback.
- Preserve a private credential recovery path if registration succeeds but writing the new handle fails, and report how to recover it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `durable-daemon`: restarting with stale metadata succeeds when an existing metadata file must be replaced.
- `agent-identity`: a committed registration remains recoverable if saving its new private handle fails.

## Impact

Affected code includes `src/store.rs`, `src/daemon.rs`, `src/integration.rs`, `src/cli.rs`, and `src/client.rs`. No wire protocol or dependency change is intended.
