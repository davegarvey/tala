# tala Eval Framework

Evaluate tala by running sub-agents through realistic multi-agent scenarios.
The script (`eval/eval-loop.sh`) orchestrates the full loop autonomously —
each phase invokes a separate opencode agent. The script owns the control
flow; agents do exactly one narrow task each.

## Automated eval (recommended)

```
./eval/eval-loop.sh <scenario>
```

Options:
- `MAX_LOOPS=5` — max iterations before stopping
- `AGENT_TIMEOUT=1800` — seconds per agent (default 30 min)
- `MODEL=anthropic/claude-sonnet-4-20250514` — model for all agent invocations (default: opencode's default)
- `VARIANT=max` — reasoning effort (provider-specific, e.g. `high`, `max`, `minimal`)

The script starts an `opencode serve` instance, runs through all phases
(setup → launch → collect → critique → analyze → implement → PR → merge),
and loops until exit criteria are met or max loops reached.

## Manual eval (step-by-step)

For step-by-step control, use the harness commands directly:
```
./eval/harness.sh scenario cross-project
./eval/harness.sh advance setup
...
```

See `.opencode/skills/tala-eval/SKILL.md` for full documentation.
