# Design

## Context

The current wait command repeatedly reads inbox and board state from the CLI. SQLite schema v5 stores request completion and delivery receipts but has no acceptance or durable notification cursor.

## Goals / Non-Goals

**Goals:** Preserve current identity/message data; make state changes idempotent; hold one client request while the daemon evaluates timers; recover events from SQLite after restart.

**Non-Goals:** General peer presence events, progress heartbeats, task execution monitoring, or automatic reassignment.

## Decisions

- Store request acceptance state and optional acceptance deadline on the request, and store generated notifications in an append-only per-recipient event table with monotonically increasing IDs. Decision changes and event inserts share a SQLite transaction.
- Add a daemon wait RPC carrying inbox, board and event cursors and an overall timeout. The daemon checks unread mail, board activity and durable events, then sleeps between checks internally; the CLI makes one blocking request and retries only on transport failure. Deadlines are evaluated in those same daemon checks and on pending/decision operations so downtime is recovered.
- Keep deadline expiry informational and keep requests open; allow late acceptance. Reply, cancellation and supersession suppress only future expiry creation. If expiry committed first, its historical event remains visible.
- Use schema v6 and protocol v10. Migration is additive and transactional. No handles, identities, messages or cursor files are rewritten.
- Track delivery-overdue per message with a persisted deduplication marker/event. A threshold is configurable at send time with a default; an unread message remains stored until its recipient fetches it. Inactive presence is only a hint source, never a failure state.

## Risks / Trade-offs

- A long HTTP request can be interrupted by a daemon restart; cursor replay and durable events recover notifications, while the client's existing lifecycle handling preserves stop semantics.
- A polling loop inside the daemon adds small local SQLite read traffic. It avoids client wakeups and token/CPU churn and can later use an in-process signal for immediate state changes.
- A delivery notification cannot prove a recipient has stopped working. Output must name unread delivery and elapsed time only.

## Migration Plan

Upgrade SQLite transactionally on startup, bump the CLI/daemon protocol together, stop and restart the daemon during the local binary update so it cannot keep serving the older wire protocol, and verify the existing home and identity count after upgrade. Rollback uses the prior binary only after stopping the upgraded daemon; the prior binary will reject schema v6, so preserve a database backup before any manual rollback.
