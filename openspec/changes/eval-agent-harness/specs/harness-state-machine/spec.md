## ADDED Requirements

### Requirement: Harness enforces state machine

The harness SHALL implement a deterministic state machine for the eval loop. State transitions SHALL be guarded by preconditions. The harness SHALL reject any transition whose preconditions are not met, print the unmet precondition, and remain in the current state.

States: `initial`, `launching`, `collecting`, `critiquing`, `analyzing`, `specing`, `pr_ci`, `finished`.

`setup` is NOT a reachable state — it is the action taken during the `initial → launching` or `pr_ci → launching` transition.

#### Scenario: Valid state transition succeeds
- **WHEN** the harness is in state `initial` and the user calls `advance setup`
- **THEN** the harness runs setup actions and transitions directly to state `launching`

#### Scenario: Invalid state transition is rejected with specific error
- **WHEN** the harness is in state `initial` and the user calls `advance collect`
- **THEN** the harness SHALL print `Cannot transition to 'collect'. Precondition: must be in state 'launching'` and remain in state `initial`

#### Scenario: Unknown target is rejected
- **WHEN** the user calls `advance nonexistent_state`
- **THEN** the harness SHALL print `Unknown state 'nonexistent_state'. Valid targets: setup, collecting, critiquing, analyzing, spec, exit, pr`

### Requirement: State is persisted in env-var format, parsed line-by-line

The harness SHALL persist its current state to `eval/.harness-state.env` in `KEY=VALUE` format. The harness SHALL read this file using `while IFS='=' read -r key value; do case "$key" in ... esac; done` — NOT via `source`. This prevents code injection from crafted values.

State file keys:
```
HARNESS_VERSION=1
STATE=initial
SCENARIO=
LOOP=0
MAX_LOOPS=5
HARNESS_PID=<pid>
```

Unknown keys SHALL be silently ignored. Missing required keys SHALL trigger a reset to `initial`.

#### Scenario: State survives process restart
- **WHEN** the harness is in state `critiquing` and the process is killed and restarted
- **THEN** the harness SHALL report state `critiquing` after reading `eval/.harness-state.env`

#### Scenario: Fresh start initializes clean state
- **WHEN** `eval/.harness-state.env` does not exist and the harness starts
- **THEN** the harness SHALL initialize to state `initial` with loop count 0

#### Scenario: Corrupt state file resets to initial
- **WHEN** `eval/.harness-state.env` contains `STATE=$(rm -rf /)` (malicious or corrupt)
- **THEN** the line-by-line parser SHALL treat `$(rm -rf /)` as a literal value string, SHALL NOT execute it, SHALL print a warning, and SHALL reset to `initial`

#### Scenario: Atomic write prevents mid-write corruption
- **WHEN** the harness is writing state and the process is killed
- **THEN** the temp file `.harness-state.env.tmp` SHALL be discarded on restart; the last fully-written state file SHALL be used

### Requirement: State file is gitignored

The file `eval/.harness-state.env` SHALL be listed in `.gitignore`.

#### Scenario: State file not tracked by git
- **WHEN** `git status` is run
- **THEN** `eval/.harness-state.env` SHALL NOT appear in untracked files

### Requirement: PID lock file prevents concurrent instances

The harness SHALL write its PID to `eval/.harness.pid` on startup. Before every `advance` command, the harness SHALL verify that the PID in the lock file is still the current process. If another harness instance is running, the harness SHALL print an error and exit.

Stale PID files (process died without cleanup) SHALL be detected via `kill -0` and automatically replaced.

#### Scenario: Second instance is rejected
- **WHEN** a harness instance is already running and a second instance is started
- **THEN** the second instance SHALL print `Another harness instance is running (PID <n>).` and exit with code 1

#### Scenario: Stale PID is cleaned up
- **WHEN** `eval/.harness.pid` contains a PID that is no longer alive
- **THEN** `kill -0` returns non-zero, the harness removes the stale file, writes its own PID, and continues

### Requirement: Stale .tmp files cleaned on startup

On startup, the harness SHALL remove any `.harness-state.env.tmp` files older than 1 hour.

#### Scenario: Stale tmp file is cleaned
- **WHEN** a `.harness-state.env.tmp` file from a previous crash exists and is older than 1 hour
- **THEN** the harness SHALL delete it on startup

### Requirement: Loop tracking and deterministic exit criteria

The harness SHALL track the current loop iteration number. After each cycle, the harness SHALL evaluate exit criteria by reading the structured critic output from `eval/agent-tasks/<scenario>/critic-output-loop-<N>.json`:

- If P0 count + P1 count == 0 → transition to `finished`
- If loop count >= MAX_LOOPS → transition to `finished`
- Otherwise → increment loop count and transition to `setup` (which runs the next iteration)

#### Scenario: Loop continues when there are P0/P1 items
- **WHEN** `critic-output-loop-1.json` contains `{"p0": ["item1"], "p1": [], "p2": ["wish"]}` and loop < MAX_LOOPS
- **THEN** the harness SHALL increment loop count to 2 and transition to `setup`

#### Scenario: Loop exits when P0+P1 is zero
- **WHEN** `critic-output-loop-1.json` contains `{"p0": [], "p1": [], "p2": ["wish"]}` regardless of P2 count
- **THEN** the harness SHALL transition to `finished`

#### Scenario: Loop exits at max iterations
- **WHEN** the loop count reaches MAX_LOOPS (default 5)
- **THEN** the harness SHALL transition to `finished` regardless of remaining items

#### Scenario: Iteration history is preserved
- **WHEN** loop iteration 2 completes
- **THEN** critic output SHALL be written to `critic-output-loop-2.json` and SHALL NOT overwrite `critic-output-loop-1.json`

### Requirement: Harness CLI interface

The harness SHALL expose these commands:

- `./eval/harness.sh status` — print current state, loop iteration, scenario, available transitions, and recommended next action
- `./eval/harness.sh advance <state>` — attempt transition; run state's actions; print result; print recommended next action
- `./eval/harness.sh scenario <name>` — set active scenario; validate it exists
- `./eval/harness.sh save-critic` — read JSON from stdin, validate, write to `eval/agent-tasks/<scenario>/critic-output-loop-<N>.json`
- `./eval/harness.sh reset` — reset state machine to `initial`
- `./eval/harness.sh help` — print usage

All commands SHALL exit with code 0 on success and non-zero on failure.

#### Scenario: Status shows current state and guidance
- **WHEN** the user calls `./eval/harness.sh status` in interactive mode
- **THEN** the harness prints: current state, loop iteration, active scenario, available transitions, recommended next action

#### Scenario: Advance runs actions and transitions
- **WHEN** the user calls `./eval/harness.sh advance setup` from state `initial`
- **THEN** the harness runs setup, writes `STATE=launching` to state file, prints summary, exits with code 0

#### Scenario: Advance with unmet preconditions fails
- **WHEN** the user calls `./eval/harness.sh advance collect` from state `initial`
- **THEN** the harness prints the unmet precondition, remains in `initial`, exits with code 1

#### Scenario: Save-critic validates and writes JSON
- **WHEN** valid JSON is piped to `./eval/harness.sh save-critic`
- **THEN** the harness validates the JSON has `p0`, `p1`, `p2` arrays, writes to the iteration-indexed path, and confirms

#### Scenario: Save-critic rejects invalid JSON
- **WHEN** malformed JSON is piped to `./eval/harness.sh save-critic`
- **THEN** the harness prints the parse error and does not write a file, exits with code 1

### Requirement: Configurable max loops

The harness SHALL support `EVAL_MAX_LOOPS` environment variable to override the default maximum loop count of 5.

#### Scenario: Custom max loops respected
- **WHEN** `EVAL_MAX_LOOPS=3` and the loop count reaches 3
- **THEN** the harness transitions to `finished` at the next exit criteria check

### Requirement: Auto mode outputs KEY=VALUE with file paths

When `--auto` flag is set, the harness SHALL output machine-readable `KEY=VALUE` lines. Multi-line content (agent prompts, critic prompts) SHALL NOT be embedded inline. Instead, the harness SHALL write content to a file and output `KEY_FILE=<path>`.

#### Scenario: Auto status outputs key=value pairs
- **WHEN** `./eval/harness.sh --auto status` is called in state `launching`
- **THEN** output SHALL be `STATE=launching LOOP=1 SCENARIO=cross-project AVAILABLE=collecting`

#### Scenario: Auto advance outputs file paths, not inline content
- **WHEN** `./eval/harness.sh --auto advance setup` completes
- **THEN** output SHALL include `TASK_PROMPT_ALPHA_FILE=eval/agent-tasks/cross-project/agent-alpha.md` and `TASK_PROMPT_BETA_FILE=eval/agent-tasks/cross-project/agent-beta.md`, never inline content
