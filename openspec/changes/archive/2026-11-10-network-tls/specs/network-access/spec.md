# Spec Delta

## ADDED Requirements

### Requirement: Network transport is TLS with a pinned certificate
The network listener SHALL serve only TLS 1.3 using the host's self-signed certificate. Clients SHALL accept the server only if its certificate's SHA-256 fingerprint equals the pinned fingerprint, and SHALL verify the handshake signature with that certificate's key. A fingerprint mismatch SHALL fail with `NETWORK_AUTH`. The loopback listener SHALL remain plain HTTP.

#### Scenario: Pinned connection
- **WHEN** a client connects with the host's join code
- **THEN** requests succeed over TLS

#### Scenario: Wrong fingerprint
- **WHEN** a client's pinned fingerprint does not match the host certificate
- **THEN** the command fails with `NETWORK_AUTH` before any request is sent

#### Scenario: Plain HTTP refused
- **WHEN** a client sends plain HTTP to the network listener
- **THEN** no HTTP response is returned and no operation runs

#### Scenario: Certificate without its key
- **WHEN** a server presents the pinned certificate but cannot produce a valid handshake signature with its key
- **THEN** the client rejects the connection

### Requirement: Join code
`tala network open` and `tala network status --show-token` SHALL print a join code of the form `tala://<token>@<host:port>#<fingerprint>`. `tala network connect` SHALL reject a malformed join code with `NETWORK_CONFIG`.

#### Scenario: Join with a different address
- **WHEN** a user runs `tala network connect --url=https://192.168.1.20:7676` and pastes a join code naming another address
- **THEN** the client connects to the given address with the code's token and fingerprint

## MODIFIED Requirements

### Requirement: Remote client mode
A Tala home connected with `tala network connect` SHALL read a join code from stdin, optionally override its address with `--url`, and send every daemon operation over TLS to that address with the code's access token, SHALL keep private handles in the local home, and SHALL NOT start, stop or adopt a local daemon. Connecting SHALL verify reachability, the pinned certificate, the token and the protocol version before saving, and SHALL refuse while a local daemon is live or a network window exists in the same home. Host window commands SHALL fail with `REMOTE_MODE` in a connected home. `tala network disconnect` SHALL return the home to local mode.

#### Scenario: Agents on two machines exchange messages
- **WHEN** an agent registered through a remote client sends a request to an agent registered on the host
- **THEN** the host agent receives it in its inbox and its reply reaches the remote agent

#### Scenario: Remote host unreachable
- **WHEN** the remote URL cannot be reached
- **THEN** the command fails with `DAEMON_UNREACHABLE` and no local daemon is started

#### Scenario: Local stop in remote mode
- **WHEN** a user runs `tala stop` in a home connected to a remote daemon
- **THEN** the command fails with `REMOTE_MODE` and the remote daemon keeps running

### Requirement: Access token lifecycle
The host's access token and TLS certificate SHALL be created together on the first window, persist across windows and restarts, and change together only through `tala network rotate-token`. Rotation SHALL take effect for the next network request and the next TLS handshake without closing the window. Network requests SHALL be checked against the current token.

#### Scenario: Rotate the token
- **WHEN** the host rotates its token while a client is connected
- **THEN** the client's next request fails with `NETWORK_AUTH` until it runs `tala network connect` with the new join code

### Requirement: Status reports network state without secrets
`tala status` and `tala network status` SHALL report the window (open state, bound address, expiry, remaining seconds and last listener error) on a host, and the remote URL and reachability in remote mode. They SHALL report the certificate fingerprint, and SHALL NOT include the access token or join code unless `tala network status --show-token` is used.

#### Scenario: Status on a remote client
- **WHEN** a user runs `tala status` in remote mode
- **THEN** status reports the remote URL and whether the remote daemon is reachable and compatible, without the token

### Requirement: Time-limited network window
A Tala home SHALL listen only on loopback unless a network window is open. `tala network open [--bind=IP:PORT] [--for=DURATION]` SHALL open a window on the given address (default `0.0.0.0:7676`) for the given duration (default one hour). No window deadline SHALL be more than 24 hours ahead, and a result whose deadline was capped SHALL report `capped: true`. Durations SHALL require a unit of `s`, `m`, `h` or `d`.

#### Scenario: Open a window
- **WHEN** a user runs `tala network open --bind=192.168.1.20:7676 --for=2h`
- **THEN** the daemon listens on that address for two hours, and the output includes the bound address, expiry, remaining seconds, the certificate fingerprint and a join code for `tala network connect` on the other machine

#### Scenario: Duration without a unit
- **WHEN** a user passes `--for=90`
- **THEN** the command fails with a usage error
