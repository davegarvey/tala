# Tasks

## 1. Daemon

- [x] 1.1 Add `listener::Manager` with open/extend/close/rotate/status, wall-clock expiry and grace-period disconnect.
- [x] 1.2 Add network RPC operations handled by the daemon before the store, accepted only over loopback; bump `PROTOCOL_VERSION` to 10.
- [x] 1.3 Restore an unexpired window at daemon start; remove persistent `enable-network` configuration and daemon-metadata listener fields.

## 2. CLI

- [x] 2.1 Replace the four flat network commands with the `tala network` group and duration parsing.
- [x] 2.2 Report the network window in `tala status` and `tala network status`, with the token only under `--show-token`.

## 3. Verification and docs

- [x] 3.1 End-to-end tests: open without restart, cross-home messaging, extend, cap, expiry disconnects a remote client, close, rotate-token invalidates the old token, restart restores an unexpired window, management refused over the network.
- [x] 3.2 Update README, agent guide and generated integration.
- [x] 3.3 Run fmt, clippy, tests and strict validation; sync and archive after `cli-command-groups`.
