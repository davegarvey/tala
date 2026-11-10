# Proposal

## Why

`enable-network` opens a listener that stays open until someone runs `disable-network` and restarts the daemon. On a laptop this is risky: a listener bound to `0.0.0.0` for a home network keeps listening on café, office or hotel networks after the user has forgotten about it. Changing it also requires a daemon restart, which interrupts every local agent's wait. The commands also add four top-level names for an occasional, user-run feature.

## What Changes

- **BREAKING:** replace `enable-network`, `disable-network`, `connect-remote` and `disconnect-remote` with a `tala network` group:
  - host: `tala network open [--bind=IP:PORT] [--for=DURATION]`, `tala network extend --for=DURATION`, `tala network close`, `tala network rotate-token`
  - client: `tala network connect --url=URL`, `tala network disconnect`
  - both: `tala network status [--show-token]`
- Network access becomes a time-limited window (default one hour, at most 24 hours ahead). A running daemon opens, extends and closes its network listener at runtime, without a restart and without touching loopback connections. When the window expires or is closed, the listener stops accepting connections and drops remaining connections after a short grace period.
- Expiry uses wall-clock time, so a window does not outlast its deadline because the laptop slept. A window survives a daemon restart only until its original expiry.
- Window management operations are accepted only over loopback.
- **BREAKING (wire):** `PROTOCOL_VERSION` becomes 10 for the new network operations.

## Capabilities

### Modified Capabilities

- `network-access`: time-limited windows replace persistent configuration; command names change.
- `agent-cli`: the command surface gains the `network` group and loses the four flat network commands.
- `durable-daemon`: the network listener follows the window; protocol version 10.

## Impact

New `src/listener.rs`; changes to `src/api.rs`, `src/daemon.rs`, `src/network.rs`, `src/client.rs`, `src/cli.rs` and `src/models.rs`; tests, README and agent guide. Released together with `cli-command-groups` as a major version.
