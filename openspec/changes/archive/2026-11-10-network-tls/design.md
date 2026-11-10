# Design

## Host identity

`network-identity.json` (mode 0600) holds the token, the certificate DER and the PKCS#8 key DER, hex-encoded. A single atomic file means that token, certificate and key cannot get out of step after a partial write. The certificate is self-signed (ECDSA P-256, generated with `rcgen` using `ring`) with subject alternative name `tala`. Its validity dates do not matter, because clients pin it rather than validating it against a certificate authority or the clock.

## Server

The listener is served by `axum-server` with a rustls `ServerConfig` restricted to TLS 1.3 and the `ring` provider. `axum-server`'s `Handle::graceful_shutdown(Some(grace))` stops accepting, then force-closes connections after the grace period. This replaces the earlier task abort, which could not reach per-connection tasks. Rotation reloads the running listener's `RustlsConfig` in place, so the window stays open and new connections see the new certificate.

## Client pinning

The client builds a rustls `ClientConfig` with a custom `ServerCertVerifier`:

- `verify_server_cert` accepts only an end-entity certificate whose SHA-256 digest equals the pinned fingerprint. It ignores the server name, intermediates and validity dates.
- `verify_tls13_signature` and `verify_tls12_signature` delegate to rustls's signature verification with the provider's algorithms. This is essential: without it, an attacker could replay the public certificate without holding its key.

reqwest receives the configuration through `use_preconfigured_tls`. Both use rustls 0.23 with `ring`.

A rustls certificate error during the handshake is reported as `NETWORK_AUTH` (fingerprint mismatch), not as `DAEMON_UNREACHABLE`.

## Join code

`tala://<token>@<host:port>#<fingerprint>` carries everything a client needs in one paste. It contains the token, so it is printed by `open` and by `status --show-token` only, and it is read from stdin. If the first suggested address is wrong for the client's network, `tala network connect --url=https://other:7676` overrides the address while keeping the code's token and fingerprint.

## Alternatives considered

- **Trust-on-first-use** (pin whatever certificate is seen first): one less value to copy, but the first connection is open to interception, which is exactly the shared-network case.
- **TLS with a pre-shared key derived from the token:** rustls does not support external pre-shared keys.
- **Optional TLS:** keeps a plain-HTTP path that nobody should use.
