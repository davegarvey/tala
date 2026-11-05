# Design

## Context

See proposal.md for the problem and user-visible scope. The daemon currently stores messages, generic thread participants, and board deliveries. `History` requires a thread ID, and its participant table does not distinguish conversational members from handoff recipients. Authenticated operations already execute in an immediate SQLite transaction; receipt, answer, cancellation, and supersession changes are made there.

## Goals / Non-Goals

**Goals:**

- Make list rows cheap to obtain without aggregating the full message table on every call.
- Preserve a complete, identity-scoped point-in-time result across separate CLI invocations.
- Keep handoff-only access distinct from conversational participation and preserve existing full-history behavior.

**Non-Goals:**

- Change board feed visibility, full thread history, or handoff authorization.
- Share or persist a cursor outside the local Tala database.
- Keep a snapshot alive beyond the fixed 15-minute expiry or eight-snapshot per-agent cap.

## Decisions

### Maintain thread and access summaries transactionally

Schema v5 adds a `thread_summaries` row with thread kind, originating board project, latest message ID/time, message count, and bounded text excerpt. A `thread_list_access` projection holds one row per identity/thread pair that is visible to that identity, with the latest message ID. Its `(agent, latest_message_id DESC, thread ASC)` index both scopes the query to one identity and provides the public order without scanning unrelated summaries or sorting a global result. Message ID determines activity; timestamp is copied from the row with the greatest ID and is display metadata only.

Separate indexed tables record conversational participants (sender or recipient of non-handoff messages) and direct handoff grants. The access projection is inserted or advanced for all current thread viewers whenever activity occurs, and is created when a new participant or handoff grantee gains access. The existing `participants` table remains intact for `History` authorization, avoiding a visibility change. Board list eligibility comes only from board post/reply authors in the conversational-participant table.

An unread counter keyed by `(thread, agent)` tracks direct unread messages and board deliveries. Open request rows keyed by request and agent track incoming and outgoing obligations. Each insert, receipt transition, answer, cancellation, and supersession updates these tables in the same transaction as the source message/delivery. Freshness is evaluated at snapshot time using the existing 24-hour cutoff and counterpart `active` flag.

The v4→v5 migration builds summaries, role tables, ordered access projections, and attention rows from existing messages, receipts, deliveries, and request state in one transaction. It leaves original message and receipt rows unchanged.

### Materialize immutable per-agent list snapshots

For a first page, capture `MAX(messages.id)` and one timestamp, seek the acting agent's `thread_list_access` ordering index, and materialize the full JSON row for each visible thread. Include names, roles, excerpt, unread and fresh/stale pending counts. This freezes both ordering and state that can change without a new message ID.

Rows are stored with immutable sort keys. Later pages use a stored random cursor token mapped to `(snapshot, agent, last_message_id, last_thread)` and a keyset predicate; no SQL `OFFSET` is used. Replaying a cursor returns the same page. Cursor ownership, malformed tokens, missing snapshots, expiry, and eviction all map to the same `INVALID_CURSOR` error.

Snapshots have a fixed 15-minute expiry and a maximum of eight unexpired snapshots per agent. Snapshot creation prunes expired rows and evicts that agent's oldest snapshot in insertion order before inserting a ninth. Foreign keys cascade snapshot deletion to row and cursor storage.

### Keep CLI modes distinct

Keep the existing `History { thread, after, limit }` wire operation unchanged. Add a distinct `ListThreads { cursor, limit }` wire operation and protocol 8. At the CLI, `history <thread>` continues to use `--after`; no-thread listing uses `--cursor`. Clap rejects combinations before any RPC.

### Render summaries safely

The excerpt uses the first line of the latest message's text part, truncated by Unicode scalar count; non-text parts do not expose file paths or data values. Human table cells escape control characters and bound joined participant names. JSON retains the bounded excerpt and complete public IDs/names.

## Risks / Trade-offs

- [Maintained counters can drift if a state mutation misses its update] → Centralize receipt/open-request helper updates, backfill from canonical predicates, and test every mutation path plus migration parity.
- [A large visible history makes first-page snapshot creation proportional to the number of visible threads] → Materialize indexed summaries rather than scanning message bodies; cap concurrent snapshots, keep snapshot rows compact, and verify query plans and a seeded large-history case.
- [Snapshot data consumes disk until expiry or eviction] → Use foreign-key cascades, the expiry index, and the fixed TTL/cap.
- [A cursor reused under another identity could reveal snapshot state if ownership checks are incomplete] → Resolve opaque cursor records only after authentication and require the cursor and parent snapshot agent IDs to match; use one generic public failure for every invalid state.

## Migration Plan

1. Bump protocol 7→8 and schema 4→5 in the same release.
2. Transactionally create summary, access, attention, and snapshot tables and backfill durable summaries from existing data.
3. Begin writing maintained summaries for new messages and state transitions.
4. Verify migration from v4 preserves message/receipt state and produces the same canonical unread/open/stale counts.

Rollback uses a matching protocol 7 binary only with an untouched schema 4 database backup; after schema 5 migration, an older binary must report `DATABASE_VERSION` rather than downgrade or rewrite the database.
