# tala Eval Scenarios

Reusable end-to-end scenarios for evaluating tala with real agents. Two agents
(or more) work through a task using tala, then report feedback. The findings
feed human-reviewed changes.

**This is deliberately NOT an autonomous loop.** The previous framework
(`eval-loop.sh`) was removed in #42 because it generated non-conventional
commits that starved the release pipeline and auto-merged its own PRs.
There is no looping, no commit generation, no PR creation — an orchestrator
(your agent) runs one scenario, collects the evidence, and the findings are
turned into a change proposal that a human reviews.

## Scenarios

| Scenario | Tests | Agents |
|---|---|---|
| [`cross-project`](scenarios/cross-project.md) | Basic collaboration: send/wait/history, session lifecycle | 2 |
| [`intent-protocol`](scenarios/intent-protocol.md) | Intent metadata: `--intent`, `--reply-to`, `pending` | 2 |
| [`wait-deadlock`](scenarios/wait-deadlock.md) | Waiting visibility: deadlock prevention, countdowns, hints | 2 |

## Isolation (read before running)

The daemon and all sessions are shared through `TALA_HOME` — that is the
point, it is how the agents collaborate. But several pieces of state are
**per working directory**, not per daemon:

- `.tala/active-session` — which session `send`/`wait` target without `-s`
- `.tala/cursor` / `.tala/cursors.json` — read receipts and unread counts
- `.tala/config.json` — the agent identity used as the sender name

This creates real confusion traps, all hit during eval runs:

- **Running tala from the wrong directory** (scratch root, repo root, or the
  other agent's project) silently makes you *that* directory's identity. From
  the repo root the sender becomes `tala`, polluting the conversation.
- **Two agents sharing a working directory** share identity, active session,
  and cursors — one agent's `send` marks the other's messages as read, and a
  stray send can land in the wrong conversation.
- **`--sender` mismatches** are hard errors (B004): an agent can only send as
  its own configured identity, so `--sender <other>` from the wrong project
  fails.

Rules:

1. Each agent works **exclusively from its own project directory**
   (`cd $SCRATCH/project-alpha` / `project-beta`) for the whole run.
2. Each project is `tala init`'d with a distinct name before anything else.
3. Never run tala from the scratch root or the repo while a scenario is live.
4. Target sessions explicitly with `-s <id>` when in doubt — don't rely on
   the active-session file across agents.

## Orchestration (per scenario)

Run everything in a scratch directory (`/tmp` or your temp dir — never the
repo). The scenario file contains the exact setup commands, seed files, and
full agent prompts.

```
1. SETUP      Create the project dirs and seed files from the scenario doc.
               Resolve the binary before starting any agent:
                 TALA_BIN="${TALA_BIN:-$(command -v tala)}"
                 "$TALA_BIN" --version
               Use the installed PATH binary for agent-to-agent messaging.
               Set TALA_BIN to an explicit checkout binary only when the eval
               is intentionally testing that checkout, and verify its version.
               Start the daemon:
                 TALA_HOME=$SCRATCH/.tala "$TALA_BIN" daemon &

2. LAUNCH     Copy the scenario's agent prompts into parallel sub-agents
               (one per agent, same TALA_HOME). Each agent works through its
              task and writes feedback to $SCRATCH/feedback/<agent>.md.

3. COLLECT    While the daemon is still alive, dump the transcript:
                 TALA_HOME=$SCRATCH/.tala "$TALA_BIN" list --json
                 TALA_HOME=$SCRATCH/.tala "$TALA_BIN" history --session <id> --json
              Then stop the daemon (sessions are in-memory; the transcript
              dump must happen first).

4. MEASURE    Compute the scenario's metrics from the transcript (see the
              scenario doc): message counts, first-reply latency, waits that
              expired, deadlock windows. Compare against the documented
              baseline.

5. ANALYZE    Read the feedback files + transcript. Triage every finding:
              P0 (blocks / contradicts spec), P1 (should fix), P2 (nice to
              have). Ground every finding in the transcript or feedback —
              no speculation.
```

## What to do with findings

1. **Triage** each finding (P0/P1/P2) with evidence.
2. **Fold accepted findings into a change proposal** (OpenSpec): new
   requirements into the capability specs, design decisions into `design.md`,
   work into `tasks.md`. If no change exists, create one.
3. **Implement** and validate (the OpenSpec workflow).
4. **Human review + PR**: create the branch and PR, and let a human review
   and merge. Never auto-merge, never generate commits from the eval itself.
   Commit messages stay conventional (`fix:`, `feat:`) so the release
   pipeline keeps bumping versions.
5. **Record baselines**: when a scenario measures a metric (e.g. the deadlock
   window), note the before/after numbers in the change summary so future
   runs can compare.

## Guardrails

- The eval never commits, pushes, or opens PRs by itself.
- Scenario runs happen in scratch dirs; nothing eval-related is committed
  except scenario definitions and measurement notes.
- Feedback is the source of truth; transcripts are evidence.
- Metrics are measured mechanically (timestamps, message ids), not
  impressionistically.

## Adding a scenario

1. Create `eval/scenarios/<name>.md` with: purpose/hypothesis, setup
   commands + seed files (heredocs), full agent prompts, metrics to measure,
   feedback questions, and any baseline to compare against.
2. Add a row to the table above.
