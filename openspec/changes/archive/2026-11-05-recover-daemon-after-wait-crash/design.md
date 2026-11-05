# Design

## Context

`client::ensure` serializes daemon startup with an OS file lock and performs asynchronous liveness probes while starting the service. `tala wait` polls inbox and board concurrently. If both request futures try to acquire the same startup lock after a daemon crash, a synchronous blocking lock call can block the Tokio task that must resume the lock owner.

## Goals / Non-Goals

**Goals:**
- Keep cross-process startup serialized while allowing async tasks in the current process to make progress.
- Let a concurrent request reuse a daemon once it becomes healthy.
- Bound lock wait and preserve protection against starting a second daemon when the recorded daemon is merely unreachable.

**Non-Goals:**
- Change wait cursors, notification semantics, wire protocol, or database schema.
- Break a lock held by a live startup process or kill a recorded PID.

## Decisions

- Replace blocking `lock_exclusive` with a `try_lock_exclusive` retry loop that awaits between attempts. On contention, probe the recorded daemon; return its info if healthy, preserve `DAEMON_UNREACHABLE` for non-refusal failures, and continue yielding only while startup may still be in progress.
- Bound lock acquisition by a startup deadline. If no daemon becomes healthy and the lock remains unavailable through the deadline, return a clear `DAEMON_START_FAILED` diagnostic instead of blocking indefinitely.
- Keep the lock guard in a narrow startup scope and release it on every success/error path. The client releases coordination as soon as it confirms a live daemon; long-polling RPCs run after `ensure` returns.
- Test the real multi-process scenario with an isolated `TALA_HOME`: a waiter's inbox and board calls race after daemon death while another command requests inbox data.

Alternatives considered: leaving `lock_exclusive` and adding a timeout around its blocking call cannot safely cancel a blocked OS syscall and can stall the Tokio worker. Removing the file lock would allow concurrent clients to launch multiple daemons. Breaking a held lock based on stale metadata could race a live daemon startup, so recovery waits for the OS lock to be released and reports a bounded error if it is not.

## Risks / Trade-offs

- **Repeated liveness probes add small local work while another process starts.** Use a short backoff and stop immediately when a healthy daemon appears.
- **A startup that legitimately takes longer than the deadline may fail for a competing client.** Use the existing five-second startup window and report the log path so the caller can retry safely.
- **Cross-platform lock behavior differs.** Keep the OS file lock and exercise the regression on supported CI platforms where practical.
