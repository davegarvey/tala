## Context

The existing eval loop (`eval/run.sh` + `.opencode/skills/chit-eval/SKILL.md`) relies on a coordinating LLM agent to execute an 8-step procedure. In practice, the agent skips steps, forgets to loop, can't recover from context limits, and hallucinates completed work.

An earlier design proposed a bash harness controlled by a "coordinator agent" — a constrained LLM that reads harness status and calls advance. Two rounds of red-team analysis showed this preserved the core flaw: an unreliable LLM making critical decisions. This design eliminates the coordinator agent entirely.

The harness is its own coordinator. It runs as an interactive CLI: it prints status, prints ready-to-copy Task tool snippets, and waits for explicit `advance` commands. The user provides judgment at the few subjective branch points. Everything else — state transitions, precondition checking, exit criteria — is deterministic code.

## Goals / Non-Goals

**Goals:**
- Deterministic state machine enforces correct step sequence
- State persisted in bash-safe env-var format (parsed line-by-line, not `source`d)
- Atomic writes prevent state file corruption on timeout
- Exit criteria evaluated programmatically from structured critic output (JSON)
- Interactive CLI guides the user; `--auto` mode outputs machine-readable status
- Backward compatible when `eval/.harness-state.env` does not exist
- Incremental delivery: Phase 1 = guarded transitions in run.sh, Phase 2 = harness.sh CLI, Phase 3 = auto mode

**Non-Goals:**
- Not calling the Task tool programmatically (IDE feature, can't be done from CLI)
- Not a general-purpose agent orchestration framework
- Not replacing the critic sub-agent's analytical role (still an LLM task)
- Not adding new eval scenarios

## Decisions

### D1: State file in env-var format, parsed line-by-line

**Decision**: State stored in `eval/.harness-state.env` as `KEY=VALUE` lines. Read via `while IFS='=' read -r key value; do case "$key" in ... esac; done < file`. Written via `printf > tmp && mv tmp file`.

**Rationale**: Unlike `source` (which executes the file and is vulnerable to code injection via crafted values), line-by-line parsing is safe. Each key is matched against known names; unknown keys are silently skipped. Values can contain spaces, `$`, backticks, or any character without risk. No `jq`, no `sed`, no parsing bugs.

**Schema:**
```
HARNESS_VERSION=1
STATE=initial
SCENARIO=
LOOP=0
MAX_LOOPS=5
HARNESS_PID=
```

**Alternative considered**: `source` .env file (previous revision). Rejected: red team identified it as a code execution vector — `STATE=$(rm -rf /)` in a corrupted file would execute before validation.

### D2: Atomic write pattern + stale .tmp cleanup

**Decision**: All state writes go to `.harness-state.env.tmp` first, then `mv` to `.harness-state.env`. On startup, the harness removes any stale `.harness-state.env.tmp` files older than 1 hour.

**Rationale**: `mv` across the same filesystem is atomic on POSIX. The stale cleanup prevents orphaned `.tmp` files from accumulating if the process is killed between write and rename.

**Also**: A PID lock file at `eval/.harness.pid` prevents concurrent harness instances. The harness writes its PID on startup and checks that no other harness PID is alive before proceeding. Stale PID files (process died without cleanup) are detected by `kill -0` and automatically replaced.

### D3: No coordinator agent — interactive CLI with --auto mode

**Decision**: The harness has no coordinator agent.
- **Interactive mode** (default): prints status, prints ready-to-copy Task tool snippets as file paths, recommends next action, waits for `advance` command.
- **--auto mode**: outputs machine-readable `KEY=VALUE` lines for programmatic consumption. Values are always simple scalars or file paths — never inline multi-line content. The supervising agent reads file paths and opens them for content.

**Rationale**: Red team's critical finding — replacing one unreliable LLM with another doesn't solve the problem. Interactive mode is fully deterministic. `--auto` mode reduces the supervisor's job to reading a few KEY=VALUE lines and calling `advance <state>`. If the supervisor ignores the recommendation, the precondition guard rejects it.

**Key constraint**: In `--auto` mode, the harness never embeds multi-line content in `KEY=VALUE` output. Instead it writes content to a file and outputs `KEY_FILE=<path>`. This avoids the encoding problem identified by the red team.

### D4: Deterministic exit criteria from structured critic output

**Decision**: The critic sub-agent outputs structured JSON to `eval/agent-tasks/<scenario>/critic-output-loop-<N>.json` (iteration-indexed for history):

```json
{
  "p0": [{"description": "...", "rationale": "..."}],
  "p1": [{"description": "...", "rationale": "..."}],
  "p2": [{"description": "...", "rationale": "..."}],
  "summary": "..."
}
```

The harness reads this file programmatically. If `p0.length + p1.length == 0`, exit criteria are met. Otherwise, the loop continues. The agent's subjective judgment is removed from the exit decision entirely.

**Critic output creation**: LLMs cannot write files. The `save-critic` subcommand reads stdin and writes the file:
```bash
./eval/harness.sh save-critic
# (paste critic JSON, then Ctrl+D)
```
This separates the harness from the Task tool — the user copies the critic's JSON response and pipes it into the harness.

**Rationale**: Exit must be deterministic to justify the harness architecture. Iteration-indexed files preserve history across loops for debugging and comparison.

### D5: Daemon health re-checks

**Decision**: The harness checks daemon health at two points:
1. During `setup` (existing behavior)
2. Before `collect` — verifies daemon is still alive after sub-agent execution

Health checks use a 5-second `timeout` to prevent hanging on a stuck daemon. If the daemon died mid-eval, the harness prints a warning but proceeds with collection (partial feedback is better than none).

**Rationale**: Red team noted the daemon can crash for many reasons (port conflict, OOM, segfault). The timeout prevents the harness itself from hanging on a stuck daemon.

### D6: Refactored shared library with HARNESS_MODE=1

**Decision**: Scenario functions move from `run.sh` to `eval/lib.sh`. Each function checks `HARNESS_MODE`:
- Unset or `0`: produce full human-readable output — backward compatible
- `1`: suppress banners, print structured data only

`run.sh` becomes a thin wrapper that sources `lib.sh` and dispatches commands.

**Backward compatibility**: Precondition checks in Phase 1 are conditional — they only apply when `eval/.harness-state.env` exists. When the state file is absent, `run.sh` operates exactly as it does today (no guards, no state tracking). This preserves existing scripts and workflows.

**Rationale**: The harness needs programmatic access to scenario functions. A single env-var toggle is cleaner than forking the code. Conditional preconditions mean existing users of `run.sh` see zero behavior change.

### D7: Incremental delivery in phases

**Decision**: Three independently testable phases:

**Phase 1 — Library + state file + guarded transitions**: Refactor `run.sh` into `eval/lib.sh`, add `HARNESS_MODE=1`, add line-by-line state file read/write, add PID lock, add conditional precondition checks. `run.sh` gains `--state-file` support for guarded transitions.

**Phase 2 — Harness CLI**: Add `eval/harness.sh` interactive CLI. State machine dispatch, advance commands, scenario management, status display, `save-critic` command, iteration-indexed output.

**Phase 3 — Auto mode**: Add `--auto` flag. Output `KEY=VALUE` status with file paths. Update critic prompt for structured JSON output. Wire deterministic exit criteria.

### D8: No advance_log or iteration metadata in state file

**Decision**: The state file contains 7 keys. No history log. Iteration history is preserved via iteration-indexed critic output files (`critic-output-loop-1.json`, etc.).

**Rationale**: The `advance_log` array was dead weight. Iteration-indexed files provide the same debugging benefit without corrupting the state file.

## State machine

```
initial ───── advance setup ────→ launching
  │                                    │
  │   (user launches sub-agents         │
  │    via Task tool, then)             │
  │   advance collecting                │
  │        ↓                            │
  │    collecting                        │
  │   advance critiquing                │
  │        ↓                            │
  │    critiquing                        │
  │   advance analyzing                 │
  │        ↓                            │
  │    analyzing                         │
  │    ┌───┴───┐                        │
  │ advance spec  advance exit          │
  │    ↓           ↓                    │
  │ specing      finished               │
  │    ↓                                │
  │   pr_ci                             │
  │    ↓                                │
  │ (check critic output)               │
  │    ┌───┴───┐                        │
  │ advance setup  advance exit         │
  │ (continue)      (finished)          │
  └─────────────────────────────────────┘
```

States: `initial`, `launching`, `collecting`, `critiquing`, `analyzing`, `specing`, `pr_ci`, `finished`.

`setup` is **not** a reachable state — it is the action taken during the `initial → launching` or `pr_ci → launching` transition. Single `advance setup` command runs all setup work and lands in `launching`.

Transitions and preconditions:

| Transition | From | Preconditions |
|---|---|---|
| `advance setup` | `initial`, `pr_ci` | Scenario set, no stale daemon, state file writable |
| `advance collecting` | `launching` | Feedback dir exists, daemon health check passes |
| `advance critiquing` | `collecting` | Aggregated feedback exists |
| `advance analyzing` | `critiquing` | Critic prompt generated |
| `advance spec` | `analyzing` | Critic output JSON has P0+P1 > 0 |
| `advance exit` | `analyzing`, `pr_ci` | Critic output JSON has P0+P1 == 0, or max loops reached |
| `advance pr` | `specing` | (trust-based — harness can't verify openspec work) |

## Risks / Trade-offs

- [User must still copy-paste Task tool snippets] → The harness can't call the Task tool. Mitigation: prompts are printed with markdown formatting in interactive mode; `--auto` mode outputs file paths.
- [User must manually save critic JSON via `save-critic`] → Inevitable — LLMs can't write files and the harness can't call the Task tool. Mitigation: single `save-critic` command with clear instructions, a one-step paste operation.
- [Env-var state file is limited] → The state is intentionally flat (7 keys). If complexity grows, adopt the line-by-line parser pattern with more keys.
- [Phase 1 precondition checks conditional on state file means no protection for legacy users] → Correct. Users who adopt the harness get guards; users who run bare `run.sh` get current behavior. This is intentional backward compatibility.
- [Critic may not output valid JSON] → Mitigations: prompt includes a validated example, `save-critic` validates the JSON before saving, harness falls back to manual confirmation if JSON is invalid.
- [Two harness PIDs could race] → Mitigation: PID lock file checked on every `advance` command, not just startup.
- [Stale .tmp file might be from a concurrent write] → 1-hour threshold is far longer than any write operation (sub-second). False cleanup risk is negligible.

## Open Questions

- None currently — all design questions resolved through red-team iterations.
