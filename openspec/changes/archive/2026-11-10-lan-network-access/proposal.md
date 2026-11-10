# Proposal

## Why

The daemon binds only to `127.0.0.1`, so agents on two machines cannot message each other. The motivating case is two MacBooks on the same home router, one on Wi-Fi and one on Ethernet, where an agent on one occasionally needs to talk to an agent on the other. Relaying through terminals defeats the purpose of Tala.

## What Changes

- A Tala home can opt in to a network listener with `tala enable-network [--bind=ADDR:PORT] [--rotate-token]`. The daemon then serves the existing RPC API on that address as well as on loopback. Every network request must carry a bearer access token stored privately in the home. `tala disable-network` removes the listener configuration.
- A second Tala home, typically on another machine, can use that daemon with `tala connect-remote --url=URL`, reading the token from stdin. In remote mode the CLI never starts, stops or adopts a local daemon; every command goes to the remote daemon, and private handles remain in the local home. `tala disconnect-remote` returns the home to local mode.
- `tala status` reports the listener address and state on the host, and the remote URL and reachability on a remote client. Status never prints the token.
- Over the network listener the daemon refuses `stop`, so a remote client cannot shut down the host, and `/health` requires the token, so the network does not learn the loopback instance identifier.
- New error codes: `NETWORK_AUTH` (missing or wrong token), `NETWORK_CONFIG` (invalid or conflicting network configuration) and `REMOTE_MODE` (operation unavailable for a home connected to a remote daemon).

## Non-goals

- Federation between two daemons that each keep their own store. One daemon owns the shared store; the other machine is a client.
- TLS termination inside Tala. The listener speaks plain HTTP. `https://` URLs are accepted on the client so a user can put a TLS proxy or tunnel in front; on an untrusted network the user should use a VPN such as Tailscale or WireGuard, or an SSH tunnel.
- Service discovery (Bonjour/mDNS).

## Capabilities

### New Capabilities

- `network-access`: opt-in network listener with token authentication and remote client mode.

### Modified Capabilities

- `durable-daemon`: the daemon still owns one home and always listens on loopback, and may also listen on a configured network address.
- `agent-cli`: the command surface gains `enable-network`, `disable-network`, `connect-remote` and `disconnect-remote`.

## Impact

`src/network.rs` (new configuration module), `src/daemon.rs` (second listener), `src/api.rs` (authenticated network router), `src/client.rs` (remote routing), `src/cli.rs` (commands and status), `src/models.rs` (error codes and daemon metadata), end-to-end tests, README and agent guide. The RPC body is unchanged and the loopback path is unchanged, so `PROTOCOL_VERSION` stays at 9; protocol compatibility is checked on connect and on each request as today.
