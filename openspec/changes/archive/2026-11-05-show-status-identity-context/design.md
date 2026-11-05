# Design

## Context

`status` is intentionally exempt from normal identity-required command handling and uses `client::existing`, which only contacts a currently reachable compatible daemon. Its current server response contains global daemon and message counts. Identity-bound `Whoami` and `Board` operations already return the public agent context and project board's `latest_id` without consuming deliveries.

## Goals / Non-Goals

**Goals:**
- Report selector provenance, public identity, registered project root, and the current board high-water mark.
- Keep global status diagnostics available when identity selection is absent or invalid.
- Keep status read-only and ensure it never starts the daemon.

**Non-Goals:**
- Persist a per-agent board read cursor or acknowledge any board/inbox messages.
- Show selector values, selector-file paths, or private credentials.
- Change the CLI-daemon wire protocol.

## Decisions

- Determine selector provenance before resolving it, using the existing precedence: explicit `--agent`, explicit `--agent-file`, `TALA_AGENT`, `TALA_AGENT_FILE`, then none. An invalid selector produces an `invalid` state and safe error code while status still reports daemon health.
- Resolve a valid handle locally to its credential and public agent ID. Preserve that ID when the daemon is stopped, but leave daemon-owned name/project fields null.
- Add an authenticated `client::existing_as` path that has the same liveness and protocol checks as `existing` and never calls `ensure`. Query existing `Whoami` and `Board` operations only when status found a compatible reachable daemon. This avoids changing RPC variants and avoids a protocol bump.
- Define `board_latest_id` as the project's latest board message ID at inspection time. It is a high-water mark for `tala board --after=<id>`, not a persisted read position or receipt.
- Report context-unavailable reasons distinctly: `no_identity`, `invalid_selector`, `daemon_stopped`, `stale_metadata`, `protocol_mismatch`, or `daemon_unreachable`. If the identity read succeeds but the board read fails, retain identity/project fields and mark only the board high-water mark unavailable.
- Give `--human` a dedicated compact status renderer. Escape control characters in names and paths before printing them.

## Risks / Trade-offs

- [The daemon can stop between the status, whoami, and board RPCs] → Use only no-start RPCs; retain partial fields already read and report the specific unavailable reason.
- [Status may run with malformed or conflicting selectors] → Keep daemon status successful and expose only selector source and error code, never raw values or file paths.
- [The board high-water mark can change immediately after inspection] → Label it as a point-in-time latest ID and avoid implying it is an acknowledgement or durable resume cursor.

## Migration Plan

No database migration or protocol change is required. Existing `Whoami` and `Board` RPCs are used as-is. Older compatible daemons can answer these operations; an incompatible daemon keeps the existing mismatch diagnostics and yields unavailable context.
