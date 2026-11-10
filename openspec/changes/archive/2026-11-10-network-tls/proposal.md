# Proposal

## Why

The network listener speaks plain HTTP. Anyone on the same network can read messages and capture the access token, and could impersonate the host to a client. The default bind is `0.0.0.0`, so a window opened on a shared network (café, office, hotel) exposes both. The network feature has not been released, so TLS can be made mandatory now, without a plain-HTTP compatibility mode.

## What Changes

- **BREAKING (unreleased feature):** the network listener serves only TLS 1.3. The loopback listener is unchanged.
- The host generates a self-signed certificate (ECDSA P-256) together with the access token on the first window. Both persist across windows and restarts, and `tala network rotate-token` replaces both together; the running listener reloads the new certificate.
- `tala network open` prints a single join code, `tala://<token>@<host:port>#<sha256-fingerprint>`. `tala network connect` reads the join code from stdin; `--url` optionally overrides the host address.
- The client pins the certificate's SHA-256 fingerprint instead of trusting certificate authorities, and still verifies TLS handshake signatures with the pinned certificate's key. A mismatch fails with `NETWORK_AUTH`.
- `tala network status` reports the fingerprint, and also the join code with `--show-token`.

## Non-goals

- Mutual TLS. The bearer token continues to authenticate clients.
- Certificate-authority-issued certificates or hostname validation.

## Capabilities

### Modified Capabilities

- `network-access`: TLS transport, certificate pinning and join codes.

## Impact

New dependencies `rustls` (ring provider), `rcgen` and `axum-server`, all without `aws-lc-rs`. Changes to `src/network.rs`, `src/listener.rs`, `src/client.rs`, `src/cli.rs`, tests, README and agent guide. No protocol bump: protocol 10 is unreleased.
