---
name: tala-eval
description: Run tala evaluation loops — sub-agents test tala in realistic multi-agent scenarios, feedback is triaged, and product improvements are implemented automatically. Load when you want to evaluate tala, run an eval, or iterate on product feedback.
license: MIT
compatibility: Requires tala development environment (AGENTS.md + eval/ directory)
metadata:
  author: tala
  version: "3.0"
---

# tala Evaluation Workflow

Evaluate tala by running sub-agents through realistic multi-agent scenarios.
Sub-agents use tala to communicate cross-project, then provide structured
feedback. The script (`eval/eval-loop.sh`) orchestrates the full loop
autonomously — each phase invokes a separate opencode agent via `opencode
run --attach`. The script owns the control flow; agents do exactly one
narrow task each.

The key architectural insight: **the script owns the flow, not the agent.**
The script starts an `opencode serve` instance, runs through all phases
(setup → launch → collect → critique → analyze → implement → PR → merge),
and loops until exit criteria are met or max loops reached. Each phase that
needs LLM work launches a constrained agent via `opencode run --attach`
against the shared server. The agent's job is tiny and specific. The script
reads structured output (JSON from critic, feedback files from sub-agents)
to decide what to do next.

## Quick Start

```bash
# Run the full eval loop (default scenario: cross-project)
./eval/eval-loop.sh

# With options
MAX_LOOPS=3 AGENT_TIMEOUT=3600 ./eval/eval-loop.sh observe
```

## Architecture

```
┌──────────────────────────────────────────────┐
│           eval/eval-loop.sh                  │
│  (standalone bash orchestrator)              │
│  - starts opencode serve (headless server)   │
│  - iterates: setup → launch → collect →      │
│    critique → analyze → implement → PR/merge │
│  - reads structured output (JSON, files)     │
│  - decides: exit, loop, or implement         │
└──────────┬───────────────────────────────────┘
           │ invokes per-phase
┌──────────▼───────────────────────────────────┐
│  opencode run --attach <URL>                 │
│  (separate agent per phase)                  │
│  - launch: parallel sub-agents               │
│  - critique: classify feedback as P0/P1/P2   │
│  - implement: spec + red-team + code         │
└──────────┬───────────────────────────────────┘
           │ sources
┌──────────▼───────────────────────────────────┐
│           eval/harness.sh                    │
│  (deterministic bash state machine)          │
│  - 7 states, 8 guarded transitions           │
│  - PID lock, atomic state file               │
│  - precondition checks on every step         │
│  - deterministic exit via structured JSON    │
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
| `cross-project` | 2 (Alpha + Beta) | Two agents collaborate across projects via tala |
| `observe` | 4 (Alpha + Beta + Gamma + Monitor) | Agents work independently; monitor watches via `tala listen` |

## Eval Loop Phases

The script runs each phase in order, looping until exit criteria or max loops:

| Phase | Implementation | Description |
|---|---|---|
| **setup** | bash (`harness.sh`) | Scenario preparation, daemon start, prompt generation |
| **launch** | `opencode run` (×N, parallel) | Sub-agents execute tasks and write feedback files |
| **collect** | bash (`harness.sh`) | Aggregate feedback, stop daemon |
| **critique** | `opencode run` (×1) | Classify feedback as P0/P1/P2, return JSON via code block |
| **analyze** | bash (`jq`) | Parse P0+P1 count; decide exit or fix |
| **implement** | `opencode run` (×1) | Create openspec change, spec, red-team, implement, commit |
| **finalize** | bash (`git` + `gh`) | Branch, push, PR, auto-merge squash |
| **exit** | bash (`harness.sh`) | Advance to finished state |

The script extracts critic JSON from the agent's output code block, validates
it via `harness.sh save-critic`, and evaluates exit criteria programmatically:
P0+P1 == 0 → exit; otherwise → implement and loop.

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

The script evaluates programmatically: P0+P1 == 0 → exit criteria met.
Max loops is the backstop. No LLM judgment in the exit decision.

## Adding a New Scenario

1. Create `eval/scenarios/<name>.md` with agent tasks and setup instructions
2. Add `setup_<name>`, `collect_<name>`, and `critique_<name>` functions to `eval/run.sh`
3. The harness auto-discovers scenarios from `eval/scenarios/*.md`

## Reference

- `eval/eval-loop.sh` — standalone orchestrator script (entry point)
- `eval/harness.sh` — deterministic state machine (interactive + --auto mode)
- `eval/lib.sh` — shared library (state file, PID lock, daemon lifecycle)
- `eval/run.sh` — thin wrapper (backward compatible, sourced by harness)
- `eval/scenarios/` — scenario definitions
- `.opencode/skills/tala/SKILL.md` — end-user tala skill
- `.opencode/commands/eval.md` — opencode command to run `eval-loop.sh`
