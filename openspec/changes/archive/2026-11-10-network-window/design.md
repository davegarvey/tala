# Design

## Ownership of state

The daemon owns the host-side network state. The CLI sends loopback RPCs (`network_open`, `network_extend`, `network_close`, `network_rotate_token`, `network_status`); the daemon binds and unbinds, and persists two private files in the home:

- `network-token`: the bearer token, created on first open and kept until `rotate-token`. Clients keep working across windows without reconnecting.
- `network.json`: the current window (`bind`, `expires_at` in Unix seconds), written on open/extend and removed on close or expiry.

A single writer avoids races between CLIs and the daemon. The CLI reads `network-token` directly only to print it (`open`, `status --show-token`).

## Listener manager

`listener::Manager` holds the running listener (bind, bound address, expiry, shutdown sender, task handle) behind an async mutex, and the current token behind a shared lock that the network router reads on each request. This means rotation takes effect immediately.

- **Open on the same bind** updates the expiry without rebinding.
- **Open on a different bind** binds the new address first. If that fails because the old listener holds the port, the old listener is closed and the bind retried. A failed open returns `NETWORK_CONFIG` with the OS reason, and loopback is unaffected.
- **Close** signals graceful shutdown (stop accepting; let in-flight requests finish) and aborts the task after three seconds. This disconnects idle keep-alive connections. Remote clients use short requests, including `wait`, which polls, so they see `DAEMON_UNREACHABLE` on their next call. The hint says the host's window may have closed.
- **Expiry**: a task checks the wall-clock deadline every second. Tokio's monotonic timers do not advance while macOS sleeps, so a plain timer would let the window outlast its deadline after sleep.

## Durations

`--for` takes an integer with a unit: `s`, `m`, `h` or `d` (for example `90m` or `2h`). The default is `1h`. `extend` adds the duration to the remaining time. The expiry is capped at 24 hours from now, and the result reports `capped: true` when the cap applied. The cap keeps the window temporary; a user who needs longer extends it deliberately.

## Restart behaviour

On start, the daemon restores an unexpired window from `network.json` and deletes an expired one. If restoring fails to bind, the daemon logs the error, keeps serving loopback, reports the error in `network status`, and removes the window. A window therefore never outlives its deadline, and `tala stop` does not silently close it. Closing is always explicit, or happens on expiry.

## Alternatives considered

- **Keep persistent configuration with an optional timeout.** This would keep two models; the timed window covers both uses, since a user can extend it.
- **Per-window tokens.** These are slightly safer, but force the client to reconnect for every window. `rotate-token` covers a suspected leak.
