## ADDED Requirements

### Requirement: Shared library with programmatic output mode

The existing scenario setup, collect, and critique functions SHALL be refactored into `eval/lib.sh`. Each function SHALL detect `HARNESS_MODE=1` environment variable and, when set, suppress human-readable banners and status lines, outputting only structured machine-readable data.

Functions that SHALL live in `eval/lib.sh`:
- `check_daemon_health(pid_file, chit_home)` — with 5-second timeout
- `show_chit_version()`
- `stop_daemon()`
- `clean_scenario(name)`
- `feedback_dir_for(scenario)`
- `collect_feedback(scenario)` — with `HARNESS_MODE=1` support
- `critique_generate(scenario, title, specifics)` — with `HARNESS_MODE=1` support

`eval/run.sh` SHALL source `eval/lib.sh` and remain backward compatible when `eval/.harness-state.env` does not exist.

#### Scenario: run.sh continues to work standalone when no state file exists
- **WHEN** `eval/.harness-state.env` does not exist and `./eval/run.sh setup cross-project` is called
- **THEN** it SHALL produce the same output as before (no precondition checks)

#### Scenario: run.sh gains guarded transitions when state file exists
- **WHEN** `eval/.harness-state.env` exists and `./eval/run.sh collect cross-project` is called without prior setup
- **THEN** it SHALL print `State file indicates state 'initial'. Cannot collect without setup.` and exit with code 1

#### Scenario: HARNESS_MODE=1 suppresses banners
- **WHEN** `HARNESS_MODE=1 ./eval/run.sh setup cross-project` is called
- **THEN** it SHALL suppress "===" banner lines and "Next step:" lines, outputting only essential data

### Requirement: Setup creates scenario environment

When the harness transitions through `setup`, it SHALL:
- Clean temp project directories only (preserve previous iteration's feedback and critic output files)
- Create temp project directories per scenario definition
- Write seed files to project directories
- Write agent task prompt files to `eval/agent-tasks/<scenario>/`
- Create feedback directory at `eval/agent-tasks/<scenario>/feedback/`
- Start chit daemon with `nohup` + `disown`
- Verify daemon health via `chit list` with 5-second `timeout`
- Print daemon PID and chit version

#### Scenario: Setup succeeds
- **WHEN** `advance setup` is called for scenario `cross-project`
- **THEN** temp dirs exist, seed files exist, task prompts exist, daemon is running and responds to `chit list`

#### Scenario: Setup fails on daemon health check
- **WHEN** setup runs and the daemon fails to respond to `chit list` within 5 seconds
- **THEN** the harness prints an error and exits with code 1 without transitioning state

### Requirement: Daemon health re-check with timeout before collect

The harness SHALL verify the chit daemon is still healthy before running the `collect` transition. The health check SHALL use a 5-second `timeout`. If the daemon died during sub-agent execution, the harness SHALL print a warning that feedback may be incomplete.

#### Scenario: Daemon alive at collect time
- **WHEN** `advance collect` is called and daemon responds to `chit list`
- **THEN** collect proceeds normally

#### Scenario: Daemon dead at collect time
- **WHEN** `advance collect` is called and daemon does not respond to `chit list`
- **THEN** harness prints "WARNING: Daemon is not running. Sub-agent communication may have been affected. Feedback may be incomplete. Proceeding with collection."

#### Scenario: Daemon hung at collect time
- **WHEN** `advance collect` is called and daemon hangs (no response)
- **THEN** the 5-second `timeout` kills the health check, harness prints the warning, and proceeds with collection

### Requirement: Collect gathers feedback and stops daemon

When the harness transitions through `collect`, it SHALL:
- Verify daemon health (see above)
- Stop the chit daemon
- Read all feedback files from `eval/agent-tasks/<scenario>/feedback/`
- Aggregate feedback into `eval/agent-tasks/<scenario>/aggregated-feedback.md`
- Print summary of collected feedback

#### Scenario: Collect succeeds with complete feedback
- **WHEN** `advance collect` is called and all expected feedback files exist
- **THEN** daemon is stopped, feedback is aggregated, aggregated file is written

#### Scenario: Collect warns on missing feedback files
- **WHEN** `advance collect` is called and some expected feedback files are missing
- **THEN** harness prints "WARNING: Missing feedback from: agent-alpha" but still aggregates any present feedback and transitions

### Requirement: Critique generates structured-output prompt

When the harness transitions through `critique`, it SHALL:
- Read aggregated feedback from `eval/agent-tasks/<scenario>/aggregated-feedback.md`
- Generate a critic prompt that includes the exact JSON schema for structured output
- The prompt SHALL instruct the critic sub-agent to return the JSON inline in its Task result
- Write the prompt to `eval/agent-tasks/<scenario>/critic-prompt.md`

#### Scenario: Critique generates structured prompt
- **WHEN** `advance critique` is called after collect
- **THEN** `critic-prompt.md` exists and includes the `{"p0":..., "p1":..., "p2":...}` schema

### Requirement: Available scenarios auto-discovered

The harness SHALL discover available scenarios by scanning `eval/scenarios/*.md` and extracting the scenario name from each filename (strip `.md` extension).

#### Scenario: Scenarios listed
- **WHEN** `./eval/harness.sh scenario list` is called
- **THEN** the harness prints all scenario names found in `eval/scenarios/`

#### Scenario: Unknown scenario rejected
- **WHEN** `./eval/harness.sh scenario nonexistent` is called
- **THEN** the harness prints `Unknown scenario 'nonexistent'. Available: cross-project, observe` and exits with code 1
