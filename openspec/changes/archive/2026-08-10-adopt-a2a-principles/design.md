## Context

tala is a single binary (CLI + hidden `daemon` subcommand) speaking ad-hoc REST + SSE over loopback, with an in-memory `Store` persisted as full-file JSON (`sessions.json`, `messages.json`, `daemon.json`). `Message` carries `content: String`; `SendMessageRequest` carries `sender` + `content`; `DaemonInfo`/`daemon.json` have no version field; `ensure_daemon_running` (cli.rs:482) attaches to a live daemon or spawns one. See proposal.md for motivation; specs/ for the behavioral contract.

Key constraint: because CLI and daemon ship as one binary, a version mismatch only occurs with a *stale daemon left running from an older binary* (or a shared `TALA_HOME`). That makes the compat check cheap insurance, not a distributed-system problem.

## Goals / Non-Goals

**Goals:**
- Parts as the canonical message content model, with read/write compatibility for legacy string messages.
- Retry-safe sends via a per-invocation client key, deduplicated by the daemon across daemon restarts.
- Fail-fast version mismatch detection with an actionable remedy, without touching every command handler.

**Non-Goals:**
- Not adopting the full A2A protocol, task lifecycle, agent cards, or push notifications (see proposal).
- No content-dedup feature — idempotency is transport-level, not content-level.
- No pagination, retention, or size caps for parts.

## Decisions

### D1: Parts as an internally-tagged serde enum, canonical on the message

`Part` is a tagged enum (`type` discriminator, snake_case kinds): `Text { content }`, `File { path, label? }`, `Data { value: serde_json::Value, label? }`. `Message.content: String` becomes `Message.parts: Vec<Part>` with a custom deserializer: `parts` if present, else a legacy `content` string → one text part. This gives read compat with `messages.json` written by old binaries with zero migration code.

For the opposite direction (old binaries reading new state), `Message` SHALL also serialize a derived `content` field — the text parts newline-joined — alongside `parts`. Old CLIs/daemons ignore the unknown `parts` field (serde default) and continue to work; this derived field is also what makes rollback safe (see D5).

- Alternative considered: keep `content: String` and add optional `parts` only when structured — rejected: two sources of truth for content, and `--json` consumers could never distinguish.
- Alternative considered: `parts` as a free-form JSON blob — rejected: defeats the typed rendering and snippet rules in the spec.

### D2: Idempotency key lives on the message; dedup via in-memory index built at load

`Message` gains `idempotency_key: Option<String>` (persisted with the message, so no new file and dedup survives daemon restart — required because the CLI reuses the key across daemon restarts). `SendMessageRequest` gains required `idempotency_key: String`. `Store` keeps `HashMap<(sender, key), (session_id, message_id)>`, rebuilt on load from persisted messages, checked under the existing add-message lock:

- key unknown → store message, record index, normal `NewMessage` broadcast;
- key known + canonical parts equal → no store, no broadcast, respond with the original message (its session and id) + `duplicate: true` — a retry landing on a different session reports the original session's message rather than a bare foreign id;
- key known + parts differ → 400 error naming the conflict.

Parts equality is canonical serialization of the `Vec<Part>` (serde_json's default map ordering makes `Data` values order-stable). `SendMessageResponse` gains `duplicate: bool` and echoes the stored message's id and session; the CLI prints "duplicate suppressed (msg N)" and exits 0, with `duplicate`/id/session as typed fields in `--json` mode. Key equality scope is per sender (global across sessions), per spec.

CLI side: `tala send` generates one random key per invocation (uuid crate v4 — small, standard) at arg-parse time. The send POST is retried on connection errors only (up to two additional attempts) with the same key — this is new behavior, added so the idempotency contract is actually exercised rather than hypothetical.

- Alternative considered: hash-of-content as the key — rejected in discussion: conflates transport idempotency with content dedup, and swallows legitimate identical messages.
- Alternative considered: separate `idempotency.json` persisted map — rejected: duplicating state that already lives on the message, and the map must be rebuilt from messages anyway.

### D3: Version check inside `ensure_daemon_running`, relaxed variant for read-only commands

`const PROTOCOL_VERSION: u32 = 1` in models.rs; `DaemonInfo`, `StatusResponse`, and `daemon.json` gain `protocol_version` (serde-default 0 so a stale daemon.json reads as 0 → mismatch). The check lives in `ensure_daemon_running` (which already fetches `/api/status` to test liveness): after connect, compare the *live* status version (not the on-disk claim, which can go stale if the daemon restarted externally); mismatch → clear error naming both versions and the remedy `tala stop` (or "upgrade tala"), nonzero exit, JSON error document in `--json` mode. Read-only commands (`status`, `discover`, `agents`) use an `allow_mismatch` variant that warns and proceeds — `discover` reads `daemon.json` directly and probes `/api/agents`, so it must be covered explicitly rather than assumed to route through `ensure_daemon_running`. The freshly-spawned path is compatible by construction (same binary) and the check short-circuits.

- Alternative considered: per-handler checks after `ensure_daemon_running` — rejected: touches ~every handler across 3,000 lines of cli.rs for no behavioral gain.
- Alternative considered: daemon refuses incompatible requests server-side — rejected: the daemon has no way to know the client's expected version; client-side gating is the only enforceable point, and the spec requires no state-mutating command be issued.

### D4: `--part <kind>:<value>` flags, positional content is legacy shorthand

`tala send` accepts repeatable `--part text:<v> | file:<path> | data:<json>` (split on the first colon; `data:` value must parse as JSON; unknown kind → usage error listing the kinds; `allow_hyphen_values` so values like `--flag` parse). A bare positional content argument is the legacy form: one text part. Mixing `--part` with positional content, `--message-file`, or `--stdin` → usage error, keeping the two forms unambiguous. `--part` presence counts as content everywhere content is gated: it passes the "nothing to send" gate, bypasses the piped-stdin fallback, and triggers session auto-creation exactly like a positional send. Part order = flag order.

### D5: Migration and rollback

- Upgrade: new CLI finds stale daemon → version check blocks with `tala stop` remedy; respawn is same-binary → consistent. `messages.json` loads legacy strings via D1's deserializer; new messages persist `parts` + derived `content`.
- Old CLI → new daemon: not blocked (the version check is client-side only) — the old CLI's legacy `content`-only payload is rejected with a clear 400 missing-`idempotency_key` error. This is acceptable: a clear error, and the version-check upgrade path prevents the common stale-daemon variant.
- Rollback (new state → old binary): the old binary ignores unknown `parts` fields and keeps reading the derived `content`, so text survives; but on the old binary's next persist it writes messages without `parts`, silently dropping typed parts. Accepted: rollback preserves text, loses typed parts; text-only content is the design floor.

## Risks / Trade-offs

- **Parts + derived `content` keeps two representations of the same message** → serde derives `content` at serialization time from `parts` (single code path), and dedup/rendering always read `parts`; `content` is output-only, so it cannot drift.
- **Multi-text-part messages render concatenated in old binaries** (derived `content` joins with newline) → acceptable: single-text messages (the common case) render identically, and text-part ordering is preserved within the canonical `parts`.
- **Dedup index grows with message count** → bounded by the store's message volume (no retention exists today); index entries are freed only by full state reset — accepted at this scale, note for a future retention feature.
- **New required `idempotency_key` is a breaking wire change** → mitigated by D3: old clients are stopped by the version check before they can send; this is also the precedent for bumping `PROTOCOL_VERSION`.
- **`data` parts can carry large values** → no size cap now; a cap can be added later without a spec change (error behavior only).
