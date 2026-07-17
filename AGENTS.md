# chit Eval Framework

Evaluate chit by running sub-agents through realistic multi-agent scenarios. Sub-agents use chit to communicate cross-project and then provide structured product feedback.

## The Eval Loop

```
1. Setup     →  ./eval/run.sh setup <scenario>
2. Launch    →  All agents in parallel via Task tool
3. Collect   →  ./eval/run.sh collect <scenario>
4. Analyze   →  Extract P0/P1/P2 from feedback
5. Spec      →  openspec new change "<name>"  (proposal → specs → design → tasks)
6. Implement →  Work through tasks, test after each group
7. PR & CI   →  Commit, PR, wait for CI, fix if needed, merge
8. Re-eval   →  Go to step 1 to validate fixes landed
```

## Quick Start

```bash
./eval/run.sh setup cross-project
```

Launch all agents in parallel (copy prompts from setup output). When they finish:

```bash
./eval/run.sh collect cross-project
```

Clean up when done:

```bash
./eval/run.sh cleanup
```

## Eval Scenarios

| Scenario | Agents | Description |
|---|---|---|
| `cross-project` | 2 (Alpha + Beta) | Two agents collaborate across projects via chit |
| `observe` | 4 (Alpha + Beta + Gamma + Monitor) | Agents work independently; monitor watches via `chit observe` |

## Adding a Scenario

1. Create `eval/scenarios/<name>.md` with:
   - `## Scenario` — narrative description
   - `## Setup` — expected directory structure and seed files
   - `## Agent Tasks` — one section per agent, describing their project context and goal
   - `## Feedback` — questions each agent should answer
2. Add a `setup_<name>` and `collect_<name>` function in `eval/run.sh`.

## Eval Skill

The `chit-eval` skill (`.opencode/skills/chit-eval/SKILL.md`) documents the full eval workflow for coding agents working on chit. Load it with the `skill` tool when running evaluations.
