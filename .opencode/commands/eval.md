---
description: Run the tala eval loop for a given scenario. Each phase invokes a separate opencode agent. The script owns the full flow — setup, launch, collect, critique, analyze, implement, PR, merge, and loop.
---

Run `./eval/eval-loop.sh [$ARGUMENTS]` (default scenario: cross-project).

Environment variables:
  MAX_LOOPS=5         Max iterations before stopping
  AGENT_TIMEOUT=1800    Seconds per agent (default 30 min)
  MODEL=anthropic/claude-sonnet-4-20250514  Model for all agents (default: opencode's default)
