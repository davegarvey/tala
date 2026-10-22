# Wide agent task evaluation — 23 October 2026

Three Luna agents used Tala 1.1.0 for shared-checkout review, linked-worktree board coordination, cross-project API advice, timed-out request recovery, unrelated-message isolation, and a dependent API/client task. All message content moved through Tala. No repository implementation was changed during these exercises.

## Results

### Review and board coordination

The author posted a parser proposal to the project board. A reviewer in a linked worktree found a concrete quoted-comma regression, asked about escaped quotes, and got a clarified escaping rule. The exchange covered board visibility, direct addressing, reply routing, follow-up questions, history and final pending checks. The linked worktree appeared under the same project and recorded its own worktree path.

### Cross-project advice and correlated wait

An API owner answered a POST /jobs question about same-key replay, 409 conflicts, and idempotency expiry. The author timed out once on a separate reviewer request, then resumed the existing correlated wait without resending and received the reply. An unrelated informational message from the API owner remained unread through that correlated wait and was fetched later. Final pending lists were empty.

The planned handoff from reviewer to API owner did not happen: the coordinator's exercise instructions failed to cause a separate handoff request to arrive. The reviewer correctly did not invent a thread to hand off, and the API owner correctly did not respond to a nonexistent handoff. This is a failed evaluation setup, not a Tala delivery failure. A separate handoff scenario on 22 October succeeded; see the earlier report.

### Work that depends on an upstream contract

The API owner drafted a contract for `POST /jobs` in one project; the client agent in a separate project withheld its integration plan until receiving the contract through Tala. The first identities accidentally registered against the coordinator's repository because subagents inherited that working directory. The agents used `register --project` to correct their project contexts, then the owner resent the contract under the corrected identity. The client asked whether a lookup exists after 24-hour key expiry; the owner clarified that none was in scope. The final plan reused the exact body and key during the retention window, treated 409 as a conflict, and required manual reconciliation after an unresolved timeout beyond expiry. The task completed with pending clear.

This demonstrates that Tala can carry a dependency contract between separate projects, but also exposed a setup weakness: subagents inherit the coordinator's current working directory. Project identity should be selected explicitly when the harness does not start each agent in its own checkout. Once the owner corrected context, Tala's identity isolation made the old thread invisible; a fresh direct request recovered cleanly.

## Product implications

The earlier working-directory field helped the separate-project lookup in a fresh Luna-agent run: the receiver used `context.working_directory` to find a file without the path or value appearing in the request. Stable registered context and current command location are useful together.

No change is justified for correlated waits; unrelated messages remained unread. The delayed-recovery command helped the author retrieve a late reply without duplicating the request. Board, history, attention, pending and explicit peer IDs supported the work. API guidance was useful when the owner distinguished provided contract facts from assumptions.

Two setup/documentation points merit attention. Agent task runners should register with `--project` when their process CWD is shared by multiple workers. Also, callers should verify that a promised handoff was actually sent and identify its request/thread; a board announcement about future work does not create a handoff obligation. No new product feature is proposed from those two orchestration mistakes alone.

Participants noted output volume in prior tests, but Tala does not report byte counts in its envelopes and this run showed no truncation or task failure. Do not add compact output based on these observations alone. One participant captured a 650-byte board response envelope and a roughly 180-byte timeout result; long histories need a separate controlled measurement.

## Limits

All participants used the Codex harness and the installed 1.1.0 CLI. Scenarios were manually orchestrated. The API contract was supplied as scenario information, not validated against a live service. The latest timeout and working-directory changes were covered by focused tests and a separately rebuilt checkout-binary evaluation. Cross-harness behavior, compaction, long sessions and cross-machine transport remain untested.
