# Design

## Context

`status` is intentionally exempt from normal identity-required command handling and uses `client::existing`, which only contacts a currently reachable compatible daemon. Its current server response contains global daemon and message counts. Every authenticated RPC refreshes agent presence in the store prelude, so using authenticated `Whoami` or `Board` calls from status would change the signal status is meant to inspect.

## Goals / Non-Goals

**Goals:**
- Report selector provenance, public identity, registered project root, and the current board high-water mark.
- Keep global status diagnostics available when identity selection is absent or invalid.
- Keep status read-only and ensure it never starts the daemon or refreshes agent presence.

**Non-Goals:**
- Persist a per-agent board read cursor or acknowledge any board/inbox messages.
- Show selector values, selector-file paths, or private credentials.
- Change the CLI-daemon wire protocol.

## Decisions

- Determine selector provenance before resolving it, using the existing precedence: explicit `--agent`, explicit `--agent-file`, `TALA_AGENT`, `TALA_AGENT_FILE`, then none. An invalid selector produces an `invalid` state and safe error code while status still reports daemon health.
- Resolve a valid handle locally to its credential and public agent ID. Preserve that ID when the daemon is stopped, but leave daemon-owned name/project fields null.
- Read identity and project board context from one read-only SQLite transaction, matching both public ID and token. Read the board high-water ID with the same `messages WHERE project=?` predicate as the Board operation. This avoids authenticated RPC presence updates, works while the daemon is stopped, and does not require a protocol change.
- Define `board_latest_id` as the project's latest board message ID in that database snapshot. It is a high-water mark for `tala board --after=<id>`, not a persisted read position or receipt.
- Report context-unavailable reasons distinctly: `no_identity`, `invalid_selector`, `identity_not_found`, or `storage_unavailable`. Daemon stopped/stale/incompatible/unreachable state remains in the existing daemon diagnostic fields; it does not make a readable local snapshot unavailable.
- Give `--human` a dedicated compact status renderer. Escape control characters in names and paths before printing them.

## Risks / Trade-offs

- [The local database may be locked, missing, or unreadable] → Use a bounded busy timeout and read-only connection; preserve daemon diagnostics and mark only local context unavailable.
- [Status may run with malformed or conflicting selectors] → Keep daemon status successful and expose only selector source and error code, never raw values or file paths.
- [The board high-water mark can change immediately after inspection] → Label it as a point-in-time latest ID and avoid implying it is an acknowledgement or durable resume cursor.

## Migration Plan

No database migration or protocol change is required. The read-only snapshot uses existing tables and leaves the live daemon untouched. If the database cannot be read, status keeps its existing daemon diagnostics and reports local context as unavailable.
