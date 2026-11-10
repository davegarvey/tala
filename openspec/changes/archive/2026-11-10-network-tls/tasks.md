# Tasks

- [x] 1.1 Generate, persist and rotate the host identity (token, certificate, key) atomically.
- [x] 1.2 Serve the network listener over TLS 1.3 with `axum-server`; reload on rotation; graceful shutdown with forced close after the grace period.
- [x] 1.3 Pin the certificate fingerprint on the client with signature verification; map mismatch to `NETWORK_AUTH`.
- [x] 1.4 Replace `--url`-plus-token connect with the join code; show fingerprint and join code in status.
- [x] 1.5 Tests: TLS round trip, wrong fingerprint rejected, plain HTTP rejected, verifier rejects bad signatures, rotation replaces certificate, join-code parsing.
- [x] 1.6 Update README, agent guide and generated integration; run fmt, clippy, tests and strict validation; sync and archive.
