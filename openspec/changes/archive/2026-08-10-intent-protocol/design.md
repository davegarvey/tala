## Context

See `proposal.md` — Why. Current state: `Message` is flat (`id, session_id, sender, content, timestamp`); `wait`/`send --wait` block on a broadcast channel with a timeout (src/api.rs:215) and return *any* new message — messages arriving mid-wait are already delivered immediately; the real gap is `wait --new-session`, which only sees sessions created after the wait starts (verified: a pre-existing session is invisible to it — this caused the V6 deadlock, a *scope* problem, not a polling problem). `stream`/`listen` are SSE; the wait endpoints are long-poll HTTP requests. Sessions are the topic primitive: named conversation threads.

The design is grounded in six eval experiments (`eval/results/intent-v0..v6/`): the four-tag taxonomy was affirmed by three independent runs; `[REPLY#id]` correlation removed interleaving ambiguity; the wait deadline (V5) and deadlock baseline (V6, 342s blind wait) motivated the registry and warnings.

## Goals / Non-Goals

**Goals:**
- Message metadata for intent, correlation, expectation, and deadline — machine-visible, JSON-first
- Live waiters registry in the daemon with session-scoped overlap warnings and pruning
- SSE-backed wait carrying typed events (overlap, hint, message, result), with lag recovery
- Pending view derived from obligations, surfaced in `pending` + `list`/`status`
- Backward-compatible read paths: old clients ignore new fields

**Non-Goals:**
- No topic/subscription concept beyond sessions (sessions already are topics; name-pattern waits deferred)
- No read-receipts ("seen" tracking) — delivery receipt only
- No intent inference from content (explicit declaration only, per eval findings)
- No persistence — sessions stay ephemeral in-memory
- No daemon-side unread tracking — the unread hint is computed client-side from the CLI's own cursor

## Decisions

1. **Intent as message metadata, not text markers.** The evals tested text tags (`[REQ]`) and every agent preferred a flag: "a real flag means wait could filter on it; text markers risk being dropped when messages are piped." Decision: `intent` field with four values, single-select; the message's primary role wins. A reply that also asks a question keeps `intent: reply` and carries the question in content — validated in V3/V4 ("one tag, primary role wins").

2. **Expectation as a modifier field.** The evals surfaced "reply that also expects a reply" as the one gap; the design adds `expect_reply: bool` as a message field set by `--expect-reply`, valid with `reply`/`fyi` only, rather than a 5th tag. It feeds the pending view. Rationale: agents never needed a fifth tag; a modifier preserves the validated 4-tag core. Alternatives considered (8-combo role×expectation matrix) rejected as over-engineering per V2/V3 feedback.

3. **Absolute deadline, relative rendering.** `waiting_until` is stored as absolute UTC (send time + the client's *resolved effective timeout*), computed once by the daemon. The CLI resolves the effective timeout (explicit `--timeout`, else its configured default) and sends it with the message, so the stamped deadline always matches the client's actual wait window (red-team RT-10: the daemon's 60s default must not silently stamp a shorter deadline than the client waits). Clients render remaining = deadline − now at read time. Rationale: durations go stale the moment they're sent (V6: beta read `[WAIT@120]` 222s after expiry and had to do mental math).

4. **Obligation outlives deadline.** Expired `waiting_until` renders "wait expired" but the `req` stays open in the pending view until answered or session closed. Validated twice in V5/V6 (recipient replied after expiry; sender was glad). The deadline is urgency, not validity.

5. **Waiters registry in the daemon.** A `Mutex<Vec<ActiveWait>>` guarded registry; wait tasks register on entry, deregister on exit (guard/RAII). Entries carry a deadline; the registry prunes expired entries at each registration, and a failed stream send deregisters, so crashed clients cannot leak entries (red-team RT-6). The daemon already holds every wait connection — this is pure addition to the wait path. Scoping rule: overlap = same session, or either scope is "any new session"; new-session↔new-session pairs are suppressed to avoid the O(N²) startup warning storm (RT-12). Alternatives considered: client-side registry (rejected — clients can't see each other, that's the whole problem); central coordination outside the daemon (rejected — TALA_HOME isolation is the trust boundary).

6. **Identity on waits.** The CLI already knows its sender name (`tala init` / `--sender`); wait requests now carry it (and `Wait` gains a `--sender` flag). Identity is self-reported and informational only. Without identity warnings would say "someone"; with it, "alpha is waiting on sess_ab12 (13s left)" — the actionable form both V6 agents requested.

7. **SSE-backed wait with typed events.** Waits move from long-poll to an SSE stream carrying typed events (`message`, `overlap`, `hint`, `result`). The motivation is not polling latency — message delivery mid-wait is already immediate — it is (a) carrying registry events and hints over the same connection, (b) removing per-request HTTP overhead for long waits (RT-11). The existing `/wait` long-poll response becomes a buffered single-document mode: in `--json`, the CLI buffers events and emits one `WaitResponse` document, preserving the script contract (the wire change is marked BREAKING in the proposal, RT-7). Lag recovery: on `RecvError::Lagged` (currently swallowed with `continue`, silently dropping events), the client re-syncs from the store since its cursor (RT-11).

8. **Pre-flight warnings are informational, not blocking.** Warn on overlap (name, scope, remaining time); never prevent the wait. Rationale: overlapping waits are sometimes legitimate (sequential turns); agents in the evals used hints well but resented enforced protocols. The deadlock *verdict* ("both waiting, nothing pending") requires obligations data and is deferred until the pending view ships.

9. **Unread hint computed client-side.** The daemon has no per-client read state; the CLI holds the cursor and `compute_session_unread`. The timeout hint ("N sessions with unread messages — run `tala check`") is computed by the CLI at wait registration and on timeout, from its own cursor (RT-5). This covers the V6 case — beta's `--new-session` timeout hinting at the existing unread session — without new daemon state.

## Risks / Trade-offs

- [Registry races (wait registers/deregisters concurrently)] → Mutex-guarded; registration happens before the wait loop, deregistration in a scope guard; warnings are best-effort snapshots.
- [Registry leak on client crash] → Entries carry deadlines; pruned at every registration; failed stream sends deregister. No sweep task needed beyond registration-time pruning.
- [False-positive warnings on legitimate overlaps] → Informational phrasing; no blocking; new-session↔new-session suppressed; verdict mode deferred until obligations exist.
- [Event channel lag drops messages] → On `Lagged`, re-sync from store since cursor; covered by a dedicated test task.
- [Wait wire format change breaks scripts] → Marked BREAKING in proposal; `--json` mode buffers to a single `WaitResponse` document preserving the existing contract.
- [Old clients send messages without intent] → `intent` defaults to `fyi` on the wire; pending view and warnings simply ignore unknown intents.
- [Warnings add noise to scripts] → JSON mode emits hints as typed stream events only; human hints go to stderr.
- [reply_to id ambiguity across sessions] → `reply_to` resolves within the target session only; the pending query is per-session.

## Migration Plan

1. Ship message-intent first (fields + flags + rendering): no breaking read behavior.
2. Ship `waiting_until` + relative rendering (pure addition).
3. Ship `reply_to` + correlation fallback + pending view + `list`/`status` surfacing.
4. Ship registry + identity + warnings + hints.
5. Ship SSE-backed wait + receipt (the marked-breaking wire change).
6. Re-run `intent-v6` deadlock scenario against the shipped feature: baseline 342s, target <60s.

Rollback: each step is additive except step 5; revert step 5 by restoring the long-poll wait endpoint.

## Open Questions

- Name-pattern wait scoping (`wait --new-session --match <pattern>`) — future capability, not in scope.
- Whether the deadlock *verdict* ("both waiting, nothing pending") should be surfaced automatically once obligations exist, or remain deferred until asked for.
- Remote daemons (multi-machine TALA_HOME) and clock handling for `waiting_until` — deferred; same-machine only for now.
