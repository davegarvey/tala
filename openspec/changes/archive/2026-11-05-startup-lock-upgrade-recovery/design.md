# Design

## Context

Current clients serialize daemon startup and stop barriers through `startup.lock`. Those locks are advisory and tied to open file handles, so a newer binary cannot safely determine that a live older process is safe to interrupt. The daemon has a separate lifetime lock that already prevents two processes from owning one Tala home.

## Goals / Non-Goals

**Goals:**
- Let this and later releases start and stop independently of legacy `startup.lock` holders.
- Preserve serialization among current clients and the one-daemon-per-home invariant.

**Non-Goals:**
- Inspect, signal, or terminate another agent's process.
- Change wire protocol or wait semantics.

## Decisions

- Use a new shared coordination filename, `startup-v2.lock`, for every startup attempt and stop barrier in the current release train. This bypasses locks held on the legacy path while preserving same-generation coordination. A new filename is safer than stale-lock stealing: advisory locks cannot safely be taken from a live process, and killing it could interrupt useful work.
- Keep `daemon.lock` unchanged as the cross-version lifetime guard. If legacy and current clients race to spawn, only one daemon can acquire that lock; the loser observes or reports the existing daemon through the normal health checks.
- Add an end-to-end regression that holds `startup.lock` while a command starts a daemon, and update lifecycle barrier tests to hold `startup-v2.lock`.

## Risks / Trade-offs

- A race between old and current starters can make one startup attempt lose the daemon lock. The daemon lifetime lock prevents duplicate daemons, and a retry can reach the winner.
- A rollback to a release that only knows `startup.lock` will not coordinate its stop barrier with new `startup-v2.lock` starters. The daemon lifetime lock still prevents duplicate daemon ownership; restoring the current release restores shared coordination.

## Migration Plan

Ship the new lock path in the next release. Keep the legacy lock file untouched so existing processes can finish naturally. Future releases continue using `startup-v2.lock`; change the lock generation only if a later incompatibility requires another isolation boundary.
