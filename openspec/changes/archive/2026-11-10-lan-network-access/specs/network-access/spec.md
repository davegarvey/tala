# Spec Delta

## ADDED Requirements

### Requirement: Opt-in network listener
A Tala home SHALL listen only on loopback unless the user runs `tala enable-network`. When network access is enabled, the daemon SHALL also serve the RPC API on the configured address (default `0.0.0.0:7676`), keep the loopback listener unchanged, and record the bound address or bind error in daemon metadata. A failed network bind SHALL NOT prevent the loopback listener from serving.

#### Scenario: Network bind fails
- **WHEN** the configured network address cannot be bound
- **THEN** local commands still succeed through loopback and `tala status` reports the network error

### Requirement: Network configuration commands
`tala enable-network` SHALL store the bind address and a private access token in the home, and `tala disable-network` SHALL remove that configuration. Changes SHALL take effect on the next daemon start, and both commands SHALL report when a restart is required.

#### Scenario: Enable network access
- **WHEN** a user runs `tala enable-network --bind=192.168.1.20:7676`
- **THEN** the home stores the bind address and a private access token, and the output includes the token and a `connect-remote` command to run on the other machine

#### Scenario: Token is stable unless rotated
- **WHEN** `enable-network` runs again without `--rotate-token`
- **THEN** the existing token is kept, and with `--rotate-token` a new token replaces it

### Requirement: Network requests are authenticated
Every request to the network listener, including `/health`, SHALL carry the access token as a bearer token. The daemon SHALL compare it in constant time, reject a missing or wrong token with `NETWORK_AUTH`, never accept the loopback instance header as network authentication, and refuse the `stop` operation over the network.

#### Scenario: Wrong token
- **WHEN** a client sends a request to the network listener with a wrong token
- **THEN** the daemon returns `NETWORK_AUTH` and performs no operation

#### Scenario: Remote stop refused
- **WHEN** a request over the network listener asks the daemon to stop
- **THEN** the daemon refuses it with `REMOTE_MODE` and keeps running

### Requirement: Remote client mode
A Tala home connected with `tala connect-remote --url=URL` SHALL send every daemon operation to that URL with the access token read from stdin, SHALL keep private handles in the local home, and SHALL NOT start, stop or adopt a local daemon. Connecting SHALL verify reachability, token and protocol version before saving, and SHALL refuse while a local daemon is live or network access is enabled in the same home. `tala disconnect-remote` SHALL return the home to local mode.

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
`tala status` SHALL report the configured bind address and the listener's bound address or error on a host, and the remote URL and reachability in remote mode. Status SHALL NOT include the access token.

#### Scenario: Status on a remote client
- **WHEN** a user runs `tala status` in remote mode
- **THEN** status reports the remote URL and whether the remote daemon is reachable and compatible, without the token
