## Why

The chit eval loop currently relies on a coordinating LLM agent to follow an 8-step procedure from a SKILL.md. LLM agents are unreliable at multi-step procedural execution — they skip steps, forget loop invariants, go off-course mid-workflow, and fail to correctly evaluate exit criteria. This makes the eval loop fragile and high-touch, defeating the goal of automated iteration.

## What Changes

- **New `eval/harness.sh` script** — a deterministic state machine that manages the full eval loop (setup → launch → collect → critique → analyze → spec → implement → PR/CI → loop/exit). The harness runs as an interactive CLI: it prints status, prints ready-to-copy Task tool snippets (as file paths), and advances on user command. All control flow is handled by code, not prompts.
- **State persistence** — `eval/.harness-state.env` (bash-safe env-var format, parsed line-by-line, never `source`d) saves current state, loop count, scenario, and PID. Atomic write pattern (`.tmp` + `mv`) prevents corruption on timeout.
- **PID lock file** — `eval/.harness.pid` prevents concurrent harness instances. Stale PID detection via `kill -0`.
- **Deterministic exit criteria** — critic sub-agent outputs structured JSON (P0/P1/P2 item lists). The harness reads these via the `save-critic` subcommand and evaluates exit criteria programmatically. Exit triggers when P0+P1 count is zero or max loops are exceeded.
- **No coordinator agent** — the harness is its own coordinator. Interactive mode prints instructions and waits for `advance`. `--auto` mode outputs `KEY=VALUE` pairs (multi-line content via file paths, never inline).
- **Guarded transitions** — each state transition checks preconditions (e.g., can't `collect` before `launch` completes). Preconditions are conditional in Phase 1 (only apply when state file exists) for backward compatibility.
- **Incremental delivery** — Phase 1: library refactor + state file + guarded transitions. Phase 2: interactive CLI. Phase 3: auto mode.
- **Updated chit-eval SKILL.md** — rewritten to describe the harness-driven workflow.

## Capabilities

### New Capabilities
- `harness-state-machine`: Core state machine — states, guarded transitions, env-var persistence (line-by-line parse, not source), PID lock, stale .tmp cleanup, loop tracking, deterministic exit criteria.
- `interactive-cli`: Interactive and auto-mode CLI — status, advance, scenario, save-critic, reset commands. Auto mode outputs KEY=VALUE with file paths for multi-line content.
- `scenario-lifecycle`: Setup/collect/critique workflow — daemon lifecycle (health re-checks with timeout), task prompt generation, feedback aggregation, structured critic prompt generation. Refactored into `eval/lib.sh` with `HARNESS_MODE=1` toggle.

### Modified Capabilities
- None (first version of the harness)

## Impact

- `eval/harness.sh` — new file (~400-600 lines), state machine + interactive CLI
- `eval/lib.sh` — new file, refactored shared functions from `run.sh` with `HARNESS_MODE=1`
- `eval/run.sh` — thinned to source `lib.sh` and dispatch (backward compatible)
- `eval/.harness-state.env` — runtime state file (gitignored)
- `eval/.harness.pid` — PID lock file (gitignored)
- `.opencode/skills/chit-eval/SKILL.md` — rewrite to harness workflow
- `.gitignore` — add state and PID files
