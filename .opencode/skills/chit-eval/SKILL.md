---
name: chit-eval
description: Run chit evaluation loops — sub-agents test chit in realistic multi-agent scenarios, feedback is triaged, and product improvements are implemented automatically. Load when you want to evaluate chit, run an eval, or iterate on product feedback.
license: MIT
compatibility: Requires chit development environment (AGENTS.md + eval/ directory)
metadata:
  author: chit
  version: "2.0"
---

# chit Evaluation Workflow

Evaluate chit by running sub-agents through realistic multi-agent scenarios. Sub-agents use chit to communicate cross-project, then provide structured feedback. The eval coordinator agent drives the full loop autonomously — setup, launch, collect, critique, analyze, spec, implement, and iterate — reporting progress at each step.

The key architectural insight: the **harness** (`eval/harness.sh`) is a deterministic bash state machine that enforces correct step sequencing, precondition checks, and exit criteria. The **coordinator agent** (`eval-coordinator`) drives the harness and launches sub-agents via the Task tool. This splits the work: the harness handles control flow (reliable), the coordinator handles LLM work (analysis, code generation).

## Quick Start

```bash
# In a Task tool call, invoke the coordinator agent:
task description="Run chit eval" subagent_type="general" prompt="
Load /skill chit-eval
Follow the eval coordinator agent prompt in .opencode/agents/eval-coordinator.md
"
```

Or if you prefer to drive it manually (step by step):

```bash
# Set up the scenario
./eval/harness.sh scenario cross-project
./eval/harness.sh advance setup

# Launch sub-agents (copy prompts from the generated files)
# ...

# Collect and critique
./eval/harness.sh advance collecting
./eval/harness.sh advance critiquing

# Save critic output
echo '<critic-json>' | ./eval/harness.sh save-critic
./eval/harness.sh advance analyzing

# Fix issues or exit
./eval/harness.sh advance spec   # if items found
./eval/harness.sh advance exit   # if no items
```

## Architecture

```
┌──────────────────────────────────────────────┐
│           eval-coordinator agent             │
│  (opencode subagent with bash + task tools)  │
│  - reads harness status                      │
│  - launches sub-agents via Task tool         │
│  - reports progress to user                  │
│  - loops until STATE=finished                │
└──────────┬───────────────────────────────────┘
           │ drives
┌──────────▼───────────────────────────────────┐
│           eval/harness.sh                    │
│  (deterministic bash state machine)          │
│  - 7 states, 8 guarded transitions           │
│  - PID lock, atomic state file               │
│  - precondition checks on every step         │
│  - deterministic exit via structured JSON    │
│  - auto mode: KEY=VALUE for agent parsing    │
└──────────┬───────────────────────────────────┘
           │ sources
┌──────────▼───────────────────────────────────┐
│           eval/lib.sh                        │
│  (shared functions)                          │
│  - daemon lifecycle                          │
│  - feedback collection                       │
│  - critic prompt generation                  │
│  - state file read/write                     │
│  - PID lock                                  │
└──────────────────────────────────────────────┘
```

## State Machine

```
initial ──setup──→ launching ──collecting──→ collecting
  ──critiquing──→ critiquing ──analyzing──→ analyzing
  analyzing ──spec──→ specing ──pr──→ pr_ci
  analyzing ──exit──→ finished
  pr_ci ──setup──→ (next loop)
  pr_ci ──exit──→ finished
```

## Eval Scenarios

| Scenario | Agents | Description |
|---|---|---|
| `cross-project` | 2 (Alpha + Beta) | Two agents collaborate across projects via chit |
| `observe` | 4 (Alpha + Beta + Gamma + Monitor) | Agents work independently; monitor watches via `chit observe` |

## Coordinator Agent (eval-coordinator)

Defined in `.opencode/agents/eval-coordinator.md`. This agent runs the full loop:

1. **Setup** — `harness.sh scenario <name>` + `harness.sh advance setup`
2. **Launch sub-agents** — reads prompt files, launches Task tool calls, waits
3. **Collect** — `harness.sh advance collecting`
4. **Critique** — `harness.sh advance critiquing`, launches critic via Task tool
5. **Analyze** — `harness.sh advance analyzing`, evaluates exit criteria
6. **Exit** — if no issues found, `harness.sh advance exit` → DONE
7. **Spec** — creates openspec change, red-teams via Task tool, implements
8. **PR** — `harness.sh advance pr`, loops to step 1

The agent reports progress at each step. The harness prevents skipping or misordering steps.

## Deterministic Exit Criteria

The critic sub-agent outputs structured JSON:
```json
{
  "p0": [{"description": "...", "rationale": "..."}],
  "p1": [{"description": "...", "rationale": "..."}],
  "p2": [{"description": "...", "rationale": "..."}],
  "summary": "..."
}
```

The harness evaluates programmatically: P0+P1 == 0 → exit criteria met. No LLM judgment in the exit decision.

## Adding a New Scenario

1. Create `eval/scenarios/<name>.md` with agent tasks and setup instructions
2. Add `setup_<name>`, `collect_<name>`, and `critique_<name>` functions to `eval/run.sh`
3. The harness auto-discovers scenarios from `eval/scenarios/*.md`

## Reference

- `eval/harness.sh` — deterministic state machine (interactive + --auto mode)
- `eval/lib.sh` — shared library (state file, PID lock, daemon lifecycle)
- `eval/run.sh` — thin wrapper (backward compatible, sourced by harness)
- `.opencode/agents/eval-coordinator.md` — autonomous coordinator agent
- `eval/scenarios/` — scenario definitions
- `.opencode/skills/chit/SKILL.md` — end-user chit skill
