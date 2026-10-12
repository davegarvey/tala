# Agent-first evaluation — 12 October 2026

Evaluates the agent-first change (#99) as released in Tala 1.0.0, without harness hooks, for tasks 5.4 and 8.1 of the archived `agent-first-messaging` change. Each agent pair ran as two real, separate headless sessions, Claude Code 2.1.278 (`claude -p`) and OpenCode 2.0.13 (`opencode run`, model `openai/gpt-5.6-luna`), in scratch Git checkouts with an isolated Tala home per scenario. Each agent received only `docs/agent-guide.md`, its task and permission to run shell commands; no files could be edited. Codex is not installed on this machine and was not evaluated. The interrupted-wait scenario tests CLI behaviour rather than agent judgement, so it was scripted.

Transcripts, prompts and the runner are in `eval/results/2026-10-12-agent-first/` (git-ignored).

| Measure | Shared checkout | Sibling worktrees |
|---|---:|---:|
| Wall time | 50 s | 55 s |
| Direct messages | 4 | 0 |
| Board posts / board replies | 1 / 0 | 2 / 1 |
| CLI errors | 0 | 0 |
| Wait timeouts | 0 | 1 |
| Public ID passed to `--agent` | 0 | 0 |
| Internal-storage reads | 0 | 0 |
| Unanswered requests at end | 0 | 0 |
| Messages not seen by their intended reader | 0 | 1 |
| Lost handles | untested | untested |

## Shared checkout

Both agents registered distinct identities in one directory and acted only through their own handles. Claude posted its plan, found `opencode` with `tala agents` and sent a review request; OpenCode checked bare `tala` (empty at that point), then waited with `inbox --wait` and received the request. It replied with a concrete finding (quoted separators break `line.split`), then asked a follow-up with `send --request --wait`. Claude received the review and the follow-up in one `inbox --wait`, answered, confirmed with bare `tala` that nothing was outstanding, and unregistered. Both agents reported the guide and help as sufficient.

## Sibling worktrees

Claude (main checkout) posted that it was changing `src/parser.py`; OpenCode (linked worktree) read the board, found the post on its second read, chose `src/report.py` and posted its plan. Claude polled the board in a background loop, found the plan and replied to it on the board. OpenCode then waited on its inbox for ten seconds, timed out and finished without seeing Claude's reply.

## Interrupted wait (scripted, 1.0.0)

- Indefinite `send --wait` with the daemon paused for 20 seconds: recovered and returned the reply after resume (exit 0, 23 s).
- `send --wait --timeout=10` with a 15-second pause: exit 4, `WAIT_FAILED`, `retry: never`, `stored: 3`, `next: tala inbox --wait --reply-to=3 --timeout=10`. One outgoing request remained pending.
- `daemon.log` shows no second daemon attempt. All expectations in `eval/scenarios/interrupted-wait.md` are met.

## Findings

1. **Defect: messages from a linked worktree record the main checkout.** OpenCode registered in `wt-feature` on branch `feature`, but its board post recorded `branch: main` and `worktree: …/repo`. `identity_context()` rebuilds a sender's context from its registered project path, which is now the main worktree root. This contradicts the project-board requirement that each message records its own worktree, branch and revision. The end-to-end test missed it because only the main-checkout agent posted. Fix: derive message context from the registered worktree.
2. **Replies to a board post do not reach its author.** Claude replied to OpenCode's board post; the reply went to the board thread, not OpenCode's inbox or attention, so OpenCode's `inbox --wait` could not see it. This is the only missed message in the runs.
3. **No way to wait for board activity.** Both agents polled `board` to find each other's posts (Claude in a 10-second background loop, OpenCode with `sleep 5`). Claude suggested `board --wait`.
4. **Guide gaps.** The guide does not say that `reply` works on board posts (Claude inferred it) or how to wait for a peer that has not registered yet (no `agents --wait`; repeat `agents`).
5. **Attention field unused in practice.** No agent referred to the `attention` field; both learned of work through `inbox --wait`, bare `tala` and `board`. In these short, focused runs that was sufficient.

## Evidence on hooks

No direct request was missed and no handle was lost, but no run lasted long enough for context compaction, so handle loss is untested rather than disproved. The one missed message (finding 2) was a board reply, which a hook would not have surfaced either unless board replies reached the author's attention. These runs do not support proposing harness hooks; findings 1 and 2 matter more.

## Limits

Two short runs per scenario with one model per harness; timings include model reasoning. Codex was not evaluated. Compaction and long-running sessions were not exercised.

## Re-run: sibling worktrees with fixes (same day)

Re-ran the sibling-worktrees scenario with the same prompts against a build containing the worktree-context fix (#102) and the `board-replies` change, with the updated guide. Transcripts are in `eval/results/2026-10-12-agent-first/worktrees-rerun/`.

- OpenCode's post now records its own worktree and `branch: feature` (finding 1 fixed).
- Claude used `board --wait --timeout=120` and received OpenCode's plan within seconds, with no polling loop (finding 3).
- OpenCode's `board --wait` timed out because Claude's reply arrived just before the wait started; the response's `attention` field showed one unread message with `next: tala inbox`, and OpenCode read Claude's reply from its inbox (finding 2). No message was missed. The guide now recommends passing `--after=<next_after>` so a board wait cannot skip messages posted since the previous read.
- Wall time 35 s. OpenCode reported two commands that failed on a mistyped working directory, unrelated to Tala.
