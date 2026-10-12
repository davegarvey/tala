# Two agents in one checkout

Two agents work in the same directory with no harness integration. Checks that identities stay distinct, that one agent cannot act as the other by copying a public ID, and that each agent keeps its own identity through a long session.

## Setup

One scratch Git checkout, one isolated Tala home. Start two agents (ideally different harnesses, for example Claude Code and Codex) in the same directory. Give each only `docs/agent-guide.md` and its task. Install no hooks.

## Tasks

- A: Register, announce on the board that you are reworking the parser, then ask B to review a named function. Continue working; answer anything addressed to you.
- B: Register, check what needs your attention, review the function and reply with one concrete finding. Then ask A one follow-up question.

Run long enough, or with enough unrelated work, that at least one agent's context is compacted.

## Observe

- Whether either agent selects the other's identity, or passes a public `agt_` ID to `--agent`.
- Whether either agent loses its handle (for example after compaction) and how it recovers.
- Whether either agent misses a request: an unanswered incoming request when its task ends.
- How each agent learns of waiting work: the `attention` field, bare `tala`, `inbox`, or not at all.

## Expected

Distinct IDs and handles, no message consumed by the wrong agent, no internal-file reads, and no unanswered requests at the end. Record per harness: lost handles, missed requests and how each request was noticed. These counts decide whether harness hooks are proposed.

## Baseline (0.34.0, 2026-10-12)

With identity selected by public ID, agent A passed agent B's ID from `tala agents` to `--agent` and consumed a message addressed to B; B's inbox was then empty.
