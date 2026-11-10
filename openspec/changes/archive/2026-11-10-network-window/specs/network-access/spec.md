# Spec Delta

## ADDED Requirements

### Requirement: Time-limited network window
A Tala home SHALL listen only on loopback unless a network window is open. `tala network open [--bind=IP:PORT] [--for=DURATION]` SHALL open a window on the given address (default `0.0.0.0:7676`) for the given duration (default one hour). No window deadline SHALL be more than 24 hours ahead, and a result whose deadline was capped SHALL report `capped: true`. Durations SHALL require a unit of `s`, `m`, `h` or `d`.

#### Scenario: Open a window
- **WHEN** a user runs `tala network open --bind=192.168.1.20:7676 --for=2h`
- **THEN** the daemon listens on that address for two hours, and the output includes the bound address, expiry, remaining seconds, the access token and a `tala network connect` command for the other machine

#### Scenario: Duration without a unit
- **WHEN** a user passes `--for=90`
- **THEN** the command fails with a usage error

### Requirement: Window changes apply at runtime
A running daemon SHALL open, move, extend and close its network listener without restarting and without affecting loopback connections. Opening on the bind of an already-open window SHALL update its expiry without rebinding. `tala network extend --for=DURATION` SHALL add time to the remaining window, and SHALL fail with `NETWORK_CONFIG` when no window is open. A bind failure SHALL return `NETWORK_CONFIG` with the operating system's reason and SHALL leave loopback serving.

#### Scenario: Open while local agents wait
- **WHEN** a window opens while a local agent's `tala wait` is active
- **THEN** the daemon instance is unchanged and the wait completes normally

#### Scenario: Extend the window
- **WHEN** a user runs `tala network extend --for=30m` on an open window
- **THEN** the remaining time increases by 30 minutes, up to the 24-hour cap

#### Scenario: Address in use
- **WHEN** the requested address cannot be bound
- **THEN** the command fails with `NETWORK_CONFIG`, local commands still succeed, and `tala network status` reports the last listener error

### Requirement: Window end disconnects clients
When a window expires or `tala network close` runs, the daemon SHALL stop accepting network connections immediately, allow in-flight requests a grace period of three seconds, and then drop remaining connections. Expiry SHALL be measured against wall-clock time, so that a sleeping machine does not extend a window beyond its deadline.

#### Scenario: Window expires
- **WHEN** a window's deadline passes
- **THEN** a remote client's next command fails with `DAEMON_UNREACHABLE`, and `tala network status` on the host reports the window closed

#### Scenario: Window closed explicitly
- **WHEN** the host runs `tala network close`
- **THEN** the result reports `closed: true`, remote clients are disconnected, and local agents are unaffected

### Requirement: Window survives restart only until expiry
The daemon SHALL persist the open window's bind and deadline. On start, it SHALL reopen an unexpired window with its original deadline and discard an expired one. While the daemon is stopped, `tala network status` SHALL report the window it will restore.

#### Scenario: Restart restores the window
- **WHEN** the daemon stops and restarts while a window has time remaining
- **THEN** the window reopens on the same bind with a deadline no later than the original

### Requirement: Access token lifecycle
The host's access token SHALL be created on the first window, persist across windows and restarts, and change only through `tala network rotate-token`. Rotation SHALL take effect for the next network request without closing the window. Network requests SHALL be checked against the current token.

#### Scenario: Rotate the token
- **WHEN** the host rotates its token while a client is connected
- **THEN** the client's next request fails with `NETWORK_AUTH` until it runs `tala network connect` with the new token

## MODIFIED Requirements

### Requirement: Network requests are authenticated
Every request to the network listener, including `/health`, SHALL carry the access token as a bearer token. The daemon SHALL compare it in constant time, reject a missing or wrong token with `NETWORK_AUTH`, never accept the loopback instance header as network authentication, and refuse the `stop` operation and every network-window management operation over the network.

#### Scenario: Wrong token
- **WHEN** a client sends a request to the network listener with a wrong token
- **THEN** the daemon returns `NETWORK_AUTH` and performs no operation

#### Scenario: Remote stop refused
- **WHEN** a request over the network listener asks the daemon to stop
- **THEN** the daemon refuses it with `REMOTE_MODE` and keeps running

#### Scenario: Window management refused over the network
- **WHEN** a request over the network listener asks to open, extend, close or inspect the window, or rotate the token
- **THEN** the daemon refuses it with `REMOTE_MODE`

### Requirement: Remote client mode
A Tala home connected with `tala network connect --url=URL` SHALL send every daemon operation to that URL with the access token read from stdin, SHALL keep private handles in the local home, and SHALL NOT start, stop or adopt a local daemon. Connecting SHALL verify reachability, token and protocol version before saving, and SHALL refuse while a local daemon is live or a network window exists in the same home. Host window commands SHALL fail with `REMOTE_MODE` in a connected home. `tala network disconnect` SHALL return the home to local mode.

#### Scenario: Agents on two machines exchange messages
- **WHEN** an agent registered through a remote client sends a request to an agent registered on the host
- **THEN** the host agent receives it in its inbox and its reply reaches the remote agent

#### Scenario: Remote host unreachable
- **WHEN** the remote URL cannot be reached
- **THEN** the command fails with `DAEMON_UNREACHABLE` and no local daemon is started

#### Scenario: Local stop in remote mode
- **WHEN** a user runs `tala stop` in a home connected to a remote daemon
- **THEN** the command fails with `REMOTE_MODE` and the remote daemon keeps running

### Requirement: Status reports network state without secrets
`tala status` and `tala network status` SHALL report the window (open state, bound address, expiry, remaining seconds and last listener error) on a host, and the remote URL and reachability in remote mode. They SHALL NOT include the access token unless `tala network status --show-token` is used.

#### Scenario: Status on a remote client
- **WHEN** a user runs `tala status` in remote mode
- **THEN** status reports the remote URL and whether the remote daemon is reachable and compatible, without the token

## REMOVED Requirements

### Requirement: Opt-in network listener
**Reason**: Replaced by the time-limited network window, which opens at runtime instead of from persistent configuration.
**Migration**: Use `tala network open`; the window replaces `enable-network`.

### Requirement: Network configuration commands
**Reason**: `enable-network` and `disable-network` are replaced by `tala network open|extend|close|rotate-token|status`.
**Migration**: Use the `tala network` group; no restart is required.
