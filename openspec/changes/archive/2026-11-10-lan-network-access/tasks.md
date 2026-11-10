# Tasks

## 1. Configuration and daemon

- [x] 1.1 Add `src/network.rs` with private `network.json`/`remote.json` load/save/remove, token generation, constant-time comparison and URL normalisation; verify with unit tests.
- [x] 1.2 Bind the optional network listener in the daemon with an authenticated router that refuses `stop`; record listener outcome in `daemon.json`; verify a bind failure leaves loopback serving.

## 2. Client and CLI

- [x] 2.1 Route RPCs, waits and status to the remote when `remote.json` exists, never starting a local daemon; refuse `stop` with `REMOTE_MODE`.
- [x] 2.2 Add `enable-network`, `disable-network`, `connect-remote` and `disconnect-remote`; report network state in `status` without the token.
- [x] 2.3 Add error codes `NETWORK_AUTH`, `NETWORK_CONFIG` and `REMOTE_MODE`.

## 3. Verification and docs

- [x] 3.1 End-to-end test: host home with listener on `127.0.0.1:0`, client home connected remotely, agents on both exchange a request and reply; wrong token rejected; remote `stop` refused; no daemon metadata in the client home.
- [x] 3.2 Document LAN set-up and the security trade-off in README and the agent guide.
- [x] 3.3 Run fmt, clippy `-D warnings`, tests and `openspec validate --all`.
- [x] 3.4 Sync delta specs and archive the change.
