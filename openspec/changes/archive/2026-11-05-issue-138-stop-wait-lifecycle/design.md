# Design

## Lifecycle state and lock order

Persist one atomic per-home lifecycle snapshot containing a monotonically increasing stop generation, the active daemon instance ID, a stop-in-progress flag, and an optional stopping instance ID. Protect reads and writes with a short-held lifecycle file lock. Write snapshots through the existing private atomic-file helper. A missing file means generation zero and no published instance. Hold a separate stop-operation lock throughout stop completion so a targetless stop still blocks starts and lets new waits exit immediately.

Every start attempt reserves the current generation under the lifecycle lock, releases that lock, then uses the existing `startup.lock` try-lock/yield path. While holding `startup.lock`, it reacquires the lifecycle lock and publishes the observed daemon instance only if the reservation is still current and no instance is stopping. A stale reservation aborts; it never silently reserves a newer generation. Lock nesting is only `startup.lock` then lifecycle lock. No lock is held while probing, waiting, or signaling shutdown.

Stop takes the lifecycle lock, increments the generation, marks stop-in-progress even when there is no target, records the active target as stopping, then releases it. It acquires `startup.lock` as a barrier and briefly rechecks lifecycle state. Any starter that published before stop is the target; any starter still holding or waiting for the startup lock has a stale reservation and cannot publish. Stop releases both locks before sending the existing Stop RPC and waiting for the captured instance ID to exit. It confirms the daemon lifetime lock is released before clearing state and returning success. New explicit commands wait for stop-in-progress to clear; new waits invoked while it is set return terminally instead.

Waiters bind to a generation and instance when they start. On transport loss, they check the generation before recovery; a changed generation returns `DAEMON_STOPPED`. With the generation unchanged, the existing `ensure` crash-recovery path is allowed, then the waiter rechecks under the lifecycle lock before adopting the replacement. This keeps #126 recovery while closing the stop-between-recovery-and-adoption race.

## Wait results and inbox receipts

`wait` and standalone `inbox --wait` use exit 5 with structured `DAEMON_STOPPED` on an intentional stop. Exit 3 remains the finite timeout result. A new wait first checks the stopping marker and exits 5 immediately rather than calling ordinary `ensure` and starting a replacement.

For non-consuming `wait`, event acceptance is serialized against stop by checking the bound generation under the lifecycle lock before returning the event. For consuming inbox wait, an acknowledged nonempty batch is always returned even if stop races after consumption; suppressing that response would hide messages whose receipt has already been recorded. The supervisor's next wait invocation then observes stopping and exits 5.

## Protocol compatibility

Protocol 9 fences older protocol 8 clients. The lifecycle snapshot is local state and no RPC fields are added, but older clients do not understand generation checks and can restart after transport loss. Bump both CLI and daemon protocol constants; reject mismatches before mutation. No SQLite migration is needed.

## Validation

Test concurrent start/stop reservations, targeted stop completion, wait and inbox-wait terminal behavior, event/stop/rearm races, consumed-message receipt preservation, timeout exit 3, and daemon-crash recovery with a concurrent client. Use platform-neutral process synchronization and run the existing Unix crash-recovery scenario on Unix platforms. Verify protocol 8/9 mismatch is rejected before an operation mutates storage. Update the copyable supervisor recipe to break on exit 5.
