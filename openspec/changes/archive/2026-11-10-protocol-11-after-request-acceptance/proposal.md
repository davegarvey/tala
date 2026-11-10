# Proposal

## Why

`main` released protocol 10 in v1.13.0 for request decisions and durable wait events. This branch had also claimed protocol 10 for network-window operations. After merging, a v1.13.0 daemon and this CLI would both report protocol 10 while disagreeing on the operations, so the version check would not catch the mismatch.

## What Changes

- **BREAKING (wire):** `PROTOCOL_VERSION` becomes 11, covering lifecycle coordination, thread listing, request decisions, durable wait events and network-window operations.
- Remote clients send `wait` without an overall request timeout (connecting stays bounded at five seconds), matching the local client, because the daemon now holds a wait open until its own deadline.

## Capabilities

### Modified Capabilities

- `durable-daemon`: protocol version 11.

## Impact

`src/models.rs`, `src/client.rs`, an end-to-end test and the durable-daemon spec. Stop a v1.13.0 daemon with its own binary before upgrading.
