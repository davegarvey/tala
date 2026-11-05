# Proposal

## Why

When `tala stop` races with an active or supervised wait, the waiter can mistake the intentional shutdown for a crash and start a replacement daemon. This makes stop unreliable and can leave a harness restarting waits after an operator has stopped Tala.

## What Changes

- Coordinate daemon starts and stop requests with a per-home lifecycle generation and stopping state.
- Make active `wait` and `inbox --wait` terminate with `DAEMON_STOPPED` (exit 5) after an accepted stop; preserve timeout exit 3 and crash recovery.
- Make `stop` wait for its target instance, and prevent starts reserved before stop from publishing afterward.
- Update the agent guide supervisor loop to end on exit 5 and preserve already-consumed inbox messages during a stop race.
- **BREAKING** Bump the wire protocol to 9 so older clients cannot bypass lifecycle coordination. Users must stop a protocol 8 daemon with its matching binary before replacing it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `durable-daemon`: linearizable startup/stop coordination, protocol 9 compatibility, and crash recovery behavior.
- `agent-cli`: terminal wait outcome, exit code 5, and supervisor retry behavior.

## Impact

`src/client.rs`, `src/daemon.rs`, `src/cli.rs`, `src/models.rs`, `src/store.rs`, daemon metadata/state files, `docs/agent-guide.md`, E2E/unit tests, and the durable-daemon and agent-cli specs. No SQLite migration is expected.
