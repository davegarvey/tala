---
name: chit-eval
description: Run chit evaluation loops — set up scenarios, launch sub-agents, collect feedback, fix issues, and re-validate. Load this when the user asks to evaluate chit, run an eval, or iterate on product feedback.
license: MIT
compatibility: Requires chit development environment (AGENTS.md + eval/run.sh)
metadata:
  author: chit
  version: "1.0"
---
# chit Evaluation Workflow

Evaluate chit by running sub-agents through realistic multi-agent scenarios.
Sub-agents use chit to communicate cross-project, then provide structured product feedback.

## Quick Start

```bash
# 1. Setup
./eval/run.sh setup cross-project

# 2. Launch sub-agents (copy prompts from setup output into Task tool calls)
#    - Agent Alpha: sends bug report, waits for reply
#    - Agent Beta:  reads messages, replies with fix

# 3. Collect results
./eval/run.sh collect cross-project

# 4. Clean up
./eval/run.sh cleanup
```

## Available Scenarios

| Scenario | Agents | Description |
|---|---|---|
| `cross-project` | 2 (Alpha + Beta) | Two agents collaborate across projects via chit. Alpha has a CSV parser bug, Beta has the schema docs. Tests: send, wait, recap, session management. |
| `observe` | 4 (Alpha + Beta + Gamma + Monitor) | Three agents work independently while a Monitor watches via `chit observe`. Tests: observe, filtering, multi-session awareness. |

## The Eval Loop

```
1. Setup     →  ./eval/run.sh setup <scenario>
                 Creates temp dirs, starts daemon, writes task prompts.
                 Outputs ready-to-copy Task tool prompts.

2. Launch    →  Copy prompts from terminal into Task tool calls.
                 Launch all agents in parallel (workers + monitor).

3. Collect   →  ./eval/run.sh collect <scenario>
                 Stops daemon, prints agent feedback.

4. Analyze   →  Read feedback carefully. Cross-reference agents. Extract:
                 - P0 bugs (crashes, data loss, hangs)
                 - P1 friction (confusing UX, missing features)
                 - P2 wishes (nice-to-haves)

5. Spec      →  Create an OpenSpec change for the issues:
                 openspec new change "<kebab-case-name>"
                 Follow proposal → specs → design → tasks
                 Red team the spec before implementing

6. Implement →  Work through OpenSpec tasks. Keep changes minimal.
                 Run tests after each task group:
                 cargo test --test e2e -- --test-threads=1

7. PR & CI   →  Commit, push, create PR, check CI passes.
                 gh pr create --title "..." --body "..."
                 gh pr checks --watch  # wait for CI
                 If CI fails: fix errors, amend commit, re-push
                 gh pr merge --squash --delete-branch

8. Re-eval   →  Go to step 1 to validate fixes landed.
                 Key question: did the fix address the agent's complaint?
                 If new issues emerged from the fix, add them to the backlog.
```

### CI Failure Patterns

| Failure | Likely Cause | Fix |
|---|---|---|
| Compile error | Rust type mismatch, missing import | Fix locally, amend commit |
| Test failure (e2e) | Shared `.chit/active-session` race | Rerun with `--test-threads=1` |
| Test failure (unit) | Logic change broke invariant | Update test or fix logic |
| Clippy warning | Style issue | `cargo clippy --fix` |
| Integration flake | Daemon port conflict, timeout | Rerun the job |

## Lessons from Previous Eval Rounds

### Daemon lifecycle
- Daemons die when the bash tool times out if not properly detached. Use `nohup` + `disown` to keep them alive.
- `eval/run.sh` now uses `nohup` + `disown` so setup exits cleanly even from the bash tool.
- Before re-running, kill stale daemons: `pkill -f "chit daemon"`

### CHIT_HOME
- Always set CHIT_HOME for both the daemon and sub-agents.
- The eval runner does this automatically. Sub-agents must export it.

### Active session gotchas
- `chit start` now sets the active session. Run `chit use <id>` to switch to a different one.
- Stale `.chit/active-session` in CWD can confuse parallel tests. Always use:
  `cargo test --test e2e -- --test-threads=1`

### Sub-agent tips
- Give agents a specific suggested chit workflow (concrete commands, not just goals).
- Include the exact CHIT_HOME path in the prompt.
- Launch all agents in parallel — even for observe, the Monitor should start with the workers.
- Agents self-resolve: Alpha sends, Beta waits/recaps/replies.

### Feedback analysis
- Agents often report the same issue differently. Cross-reference.
- "Frustrating" = P0. "Would be nice" = P2. 
- If an agent says `wait` didn't work, it's likely a race condition in the broadcast channel.
- If an agent couldn't discover a feature (e.g. `chit use`, session rename), the UX needs work.
- If both agents independently request the same thing (e.g. `-s` short flag for `--session`), it's a strong signal.
- If `chit wait` doesn't show the session ID on receipt, agents have to run `chit list`/`recap` separately to reply — a clear friction point.
- `chit start` silently switching the active session is confusing when agents experiment. Better to keep it scoped: create only, use `chit use` to activate.
- `chit rename` isn't a top-level command — it's `chit session rename`. Double-check command structure in task docs.
- `chit observe`'s scope (showing all sessions including the observer's own) can be surprising. Clarify in docs.
- Collecting feedback via file writes is unreliable — agents may claim to write without actually doing so. Prefer inline feedback in Task results.

### OpenSpec workflow
- Always red team the spec before implementing. Find inaccurate claims (e.g. "endpoint already supports X" when it doesn't), contradictions, and missing edge cases.
- Name resolution (`chit use <name>`) is client-side (fetch all, filter) to avoid daemon changes. Works because session count is small.
- `chit send` vs `resolve_session_id` inconsistency is intentional: send is a write (misrouted messages are silently lost), recap/close/follow are reads (safe to auto-route).
- Tests sharing CWD race on `.chit/active-session`. Isolate with `chit_in(..., Some(project_dir), ...)` or use `--test-threads=1`.

### CI after eval changes
- Always run `cargo test --test e2e -- --test-threads=1` before pushing — parallel test threads cause flaky active-session races.
- If CI fails on a flaky test, rerun the job before debugging. If it fails consistently, check for active-session file pollution from other tests.

## Adding a New Scenario

1. Create `eval/scenarios/<name>.md` with:
   - `## Scenario` — narrative description
   - `## Setup` — expected directory structure and seed files
   - `## Agent Tasks` — one section per agent
   - `## Feedback` — questions each agent should answer
2. Add `setup_<name>` and `collect_<name>` functions in `eval/run.sh`.
3. Register the scenario name in the `case` statement's help text.

## Reference

- `AGENTS.md` — high-level framework docs
- `eval/run.sh` — the orchestrator
- `eval/scenarios/` — scenario definitions
- `.opencode/skills/chit/SKILL.md` — end-user chit skill (what agents use)
- `.opencode/skills/chit-eval/SKILL.md` — this skill (what you're reading now)
