# chit Eval Framework

Evaluate chit by running sub-agents through realistic multi-agent scenarios.
The eval coordinator agent drives the full loop autonomously.

## Automated eval (recommended)

Invoke the coordinator agent to run the full loop:
```
task description="Run chit eval" subagent_type="general" prompt="
Load /skill chit-eval
Follow the eval coordinator agent prompt in .opencode/agents/eval-coordinator.md
"
```

The agent reports progress at each step. The harness (`eval/harness.sh`)
enforces correct step sequencing and prevents drift.

## Manual eval

For step-by-step control, use the harness commands directly:
```
./eval/harness.sh scenario cross-project
./eval/harness.sh advance setup
...
```

See `.opencode/skills/chit-eval/SKILL.md` for full documentation.
