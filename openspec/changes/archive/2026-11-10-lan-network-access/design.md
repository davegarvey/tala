# Design

## Context

Each Tala home owns one daemon and one SQLite store. Clients start the daemon on demand, find it through `daemon.json`, and authenticate to it with the per-start instance identifier in the `x-tala-instance` header. That identifier is a sufficient local secret only because it is in a private file and the socket is loopback-only. Agent credentials (handle → agent ID and token) live in the client's home.

## Decisions

### Hub, not federation

One daemon owns the shared store, and the other machine's CLI talks to it. Federation would need cross-daemon addressing, replication of messages and receipts, and conflict handling for the board, which is far more than the use case needs. The cost of the hub model is that the client machine depends on the host being awake and reachable. For occasional use between two laptops on one router, that cost is acceptable.

### Persistent configuration in the home, not environment variables

The daemon is started by whichever client happens to run first, and agents are launched with arbitrary environments. Configuration therefore lives in private files (mode 0600) in the home: `network.json` on the host (`bind`, `token`) and `remote.json` on the client (`url`, `token`). A home cannot be both: each command refuses if the other file exists.

### Two listeners on the host

The loopback listener is unchanged, so local agents keep the existing behaviour and security model. If `network.json` exists, the daemon also binds the configured address and serves a separate router that requires `Authorization: Bearer <token>` on `/health` and `/rpc`, never accepts the instance header as authentication, and refuses `stop`. The token is compared in constant time. If the network bind fails (for example, the port is in use), the daemon logs the error and keeps serving loopback, because local agents should not lose Tala over an optional feature. It records the outcome in `daemon.json` (`network.address` or `network.error`) so that `tala status` can report it.

The default bind is `0.0.0.0:7676`. Users can pass a specific interface address (for example `192.168.1.20:7676`) to limit exposure, or `127.0.0.1:0` in tests. Configuration changes take effect on the next daemon start; the command reports `restart_required` when a daemon is running and points to `tala stop`.

### Token handling

`enable-network` generates a 256-bit random token (two v4 UUIDs, hex) on first use and keeps it on later runs unless `--rotate-token` is passed. Its output includes the token, so that the user can transfer it, and a ready-to-run `connect-remote` command. `connect-remote` reads the token from stdin rather than from a flag, so the token stays out of process listings and shell history. Status never prints the token.

### Remote client behaviour

When `remote.json` exists, `client::call*` posts to `<url>/rpc` with the bearer token and skips local daemon discovery, startup and lifecycle generations. Waits still reserve a local lifecycle generation; nothing advances it in remote mode because `tala stop` is refused locally with `REMOTE_MODE`. Connection failures to the remote report `DAEMON_UNREACHABLE` with a network-specific hint and never start a local daemon. `connect-remote` verifies the URL, the token and the protocol version through `/health` before saving, and it refuses while a local daemon is live, because the local agents' handles would stop resolving.

### Security trade-off

Plain HTTP means that anyone on the same network segment can read message contents and the token. TLS with self-signed certificates and pinning would add dependencies, certificate lifecycle and a more complex join flow. For a trusted home LAN, an explicit opt-in, a strong token and documentation that recommends a VPN or SSH tunnel elsewhere are the proportionate choice. The trade-off is stated in the README and in the command's output.

## Known limitations

- Project identity is the checkout path. Agents on both machines working in the same absolute path share a project.
- Handles registered while connected to a remote belong to that remote's store; after `disconnect-remote` they no longer resolve against the local daemon.
- macOS asks once whether to allow incoming connections for the `tala` binary.
