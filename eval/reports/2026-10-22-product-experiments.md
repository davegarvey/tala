# Product experiments — 22 October 2026

Tala's core local communication model worked in three real-agent exercises. The clearest improvement is recovery guidance after a send times out. Preserve the identity, reply routing and outstanding-request model; these helped participants finish their work.

## Method

Three separate Codex subagents used the installed `/Users/dave/.local/bin/tala` (1.1.0, protocol 3), one isolated `TALA_HOME`, and scratch Git projects. They received `docs/agent-guide.md` and role-specific tasks, consulted CLI help as needed, and communicated task content only through Tala. The coordinator assigned phases through the agent harness; this was a bounded, manually orchestrated evaluation, not an autonomous improvement loop. Participants retained their own private handles, did not inspect internal storage, and unregistered after evidence capture.

Redacted participant notes and selected actual CLI envelopes are in `eval/results/2026-10-22-product-experiments/` (git-ignored). Initial exchanges have summarized transcripts; later exchanges include raw envelopes. Times below come from message timestamps and include agent reasoning and orchestration, not just transport latency.

## Experiments

| Exercise | Task and outcome | Evidence |
|---|---|---|
| Shared checkout | Author announced a port-parser rule; reviewer found that hex and exponent notation pass numeric conversion, asked a follow-up, and received precise input requirements. Distinct identities; both requests settled. | Messages 1–5; initial review reply 8 seconds after request, follow-up answer 20 seconds after request. |
| Cross-project question | Author changed into a client directory, kept its identity, discovered an API owner in another project, and obtained safe retry rules for POST /jobs. A separate informational update reached reviewer. | Messages 6–8; API reply in 9 seconds. |
| Timeout and delegated review | Author sent a request with a two-second wait. Reviewer peeked and read history before consuming it, handed the thread to API owner, received advice, then answered author. Author recovered the late reply without resending. | Messages 9–12; final answer 46 seconds after request, handoff answer 14 seconds after handoff. |

Across the exercises: 12 stored messages including one board post; five answer obligations, all answered; one intentional wait timeout; no reported invalid commands, wrong-recipient consumption, lost handles or internal-storage reads. Final pending lists were empty. The reviewer and API owner explicitly used attention summaries to distinguish receipt from answering. Main help was read once per participant; reviewer additionally read handoff and status help, and author read status help. Author needed one repeated discovery call before reviewer appeared.

## Product decisions

### 1. Prioritize an explicit recovery action after a timed-out send

Observed: `send --wait --timeout=2` exited 3 with `ok:true`, `timed_out:true` and the stored request under `sent.message`, but no `next` command. The guide tells agents to run `next` after exit 3 or 4. Author recovered by recalling the guide's `inbox --wait --reply-to=9` syntax.

Source inspection confirms the gap: `src/cli.rs` adds a recovery command for `WAIT_FAILED`, while the normal timeout path attaches the sent snapshot without a recovery action. This is a guidance defect, not lost delivery. The successful recovery does not remove the needless inference.

Proposed change: supply a machine-readable `next` on the timed-out send result, identifying the existing request and retaining the chosen timeout. Explain that the request remains stored and should not be resent. Keep exit 3 and the current stored-message evidence. Acceptance: a fresh agent given only the timeout result can resume the existing wait, receive its late reply, and leave exactly one original request. Cover timeout JSON and human rendering with focused tests. Prepare this as the first OpenSpec change after human review.

### 2. Clarify what message context describes before changing its behavior

Observed: author moved from shared to client, but `whoami` and outgoing message context continued to name shared. This matches the guide and `identity_context()`, which reconstructs context from the registered worktree. Author identified possible recipient confusion; API owner found the supplied project context useful. Neither failed its task.

Recommendation: clarify that context describes the registered workspace, not necessarily the invocation directory. Do not silently move identity or the default board on `chdir`. Before adding per-message current-directory context, run a focused experiment where the receiver must locate a file after the sender changes worktrees. Success should mean the receiver selects the correct file without clarification. This is a hypothesis worth testing, not a proven demand for another flag.

### 3. Measure output cost before adding a compact mode

All three participants mentioned repeated message text and context in JSON. They also used those fields successfully, especially the answered request snapshot and handoff history. No truncation or missed task was observed.

Recommendation: measure bytes/tokens and completion errors on longer threads before changing output defaults or introducing a second schema. Preserve reply IDs, request state, pagination and attention in any trial. Current evidence supports investigation, not immediate interface expansion.

### Preserve the mechanisms that worked

Private handles kept same-checkout identities distinct. Automatic reply routing reduced bookkeeping. Consuming an inbox left requests visibly outstanding. Handoff granted useful history and created its own answer obligation while the reviewer retained responsibility for the original request. Document that ownership rule more explicitly; do not change it based on these runs.

### Investigate daemon lifecycle separately

After the successful exchanges, reviewer's final `status --message=12` returned `DAEMON_NOT_RUNNING`. Following its `next: tala whoami` recovered access, and a repeated status confirmed the reply was received. A later coordinator status again reported `running:false, stale_metadata:true`. No participant reported issuing stop before these observations. The coordinator then ran stop during cleanup.

This is an unresolved reliability signal, not evidence of message loss. These runs do not distinguish daemon failure from execution-environment process cleanup. Reproduce with a persistently launched daemon outside the agent command lifecycle before proposing a fix; capture process lifetime and stderr with explicit diagnostic scope. The redacted cleanup envelopes are retained in reviewer-3.md.

## Limits and next experiments

One short run per scenario, three agents from one harness, all supplied with the guide and explicit communication tasks. No context compaction, independent background coding, cross-harness execution, restart recovery, unauthorized-history probe, large-thread pagination or cross-machine behavior was tested. Zero missed requests here is not evidence that harness hooks are unnecessary in long sessions. The unrelated message went to reviewer, so it does not test whether a correlated wait preserves unrelated messages in the same inbox.

Next run: a Claude/Codex pair doing useful independent work with an unsolicited request arriving mid-task; then a long-session identity recovery exercise. Test same-inbox correlated-wait isolation separately. Keep cross-machine transport outside the current scope until local unattended communication is better evidenced.

No product code, commits, PRs or releases were generated. This report records findings and a recommended order for human review, as required by the evaluation workflow.

## Follow-up evaluation and implementation — 23 October 2026

Two Luna agents repeated the worktree lookup before and after adding message working-directory context. In the first run, the sender registered in `shared`, moved to `worktree`, then asked for `handoff-note.txt` without naming its path. Tala continued to report the sender under `shared`; the receiver checked that location, found no file, and asked for clarification. The sender's later request still carried `/shared`, and the receiver found `amber` only after checking its own `/worktree`. The task succeeded, but took an extra round trip and exposed the misleading location.

The initial post-change run accidentally used a stale `target/debug/tala` executable. It correctly demonstrated that `working_directory` could direct the lookup, but its `project` field changed with CWD. This did not match the source change under evaluation, so that run is not treated as validation. We explicitly built the checkout binary and repeated the experiment with a fresh home. The sender registered from `/Users/dave/code/tala`, changed to `/private/tmp/tala-followup-20261023/verified/b`, and asked the receiver to find a file without supplying its path or value. The request preserved the registered project and worktree as `/Users/dave/code/tala`, recorded the new working directory as `/private/tmp/tala-followup-20261023/verified/b`, and the receiver returned `result: jade` and the correct path without clarification. The sender and receiver both used the explicitly built checkout binary, version 1.1.0. Redacted details are in `eval/results/2026-10-22-product-experiments/` for the initial runs and `/private/tmp/tala-followup-20261023/evidence/` for the follow-up runs.

This supports the added optional `context.working_directory` field. It preserves stable identity and board context while helping a peer locate resources in the sender's active checkout. The change is specified, implemented, covered by an e2e test, synced into `openspec/specs/agent-messaging/spec.md`, and archived as `2026-10-23-message-working-directory-context`.

The earlier daemon signal was not reproduced while agents were active: `status` reported the shared daemon running throughout both follow-up exchanges. An isolated registration followed by an immediate status had reported stopped/stale metadata, but there was no controlled process-lifetime capture to identify why. Keep daemon lifetime as an environment-sensitive observation, not a product defect, until a controlled reproduction identifies Tala itself as the cause.

JSON size feedback remains mixed with no demonstrated task failure. The first follow-up participant recorded `--help` at 4,308 bytes and `agents` at 990 bytes; those measurements do not establish excessive routine message output. No compact output mode is justified yet. Preserve detail and revisit only if longer exchanges show context pressure or lookup failures.

The working-directory change was followed by a fresh Luna-agent lookup on the explicitly rebuilt binary. Its focused e2e test passed; the full suite passed with 18 unit tests, 44 e2e tests and one intentionally ignored test. Formatting, Clippy with warnings denied, and OpenSpec validation passed. No cross-harness, context-compaction, or cross-machine run was performed.
