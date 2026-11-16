# Design

## Decision

Hold the store as `Arc<Mutex<Option<Store>>>`. After `axum::serve` returns, the daemon takes the store out of the mutex and drops it, which closes the SQLite connection (and checkpoints the WAL), then removes `daemon.json` and releases `daemon.lock`.

Taking the value under the mutex closes the connection regardless of how many clones of the shared state remain, for example in the network listener or its expiry task. A request already executing holds the mutex, so the close waits for it; a request arriving later sees `None` and receives `DAEMON_STOPPING`, which clients already treat as retryable.

## Alternatives considered

- Make the test wait or retry the delete. Rejected: the spec already promises that stop returns after the instance exits, and other callers (upgrades, backups, home deletion) depend on the same guarantee.
- Have the client wait for the recorded PID to exit. Rejected: the lifecycle deliberately avoids PID handling, and the lifetime lock is the established exit signal; fixing the order of release keeps one signal.
- Wait for the `Arc` strong count to reach one. Rejected: background tasks can hold clones indefinitely.
