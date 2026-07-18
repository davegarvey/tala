## Phase 1: Shared library + state file + guarded transitions

- [ ] 1.1 Extract shared functions from `run.sh` into `eval/lib.sh`: `check_daemon_health`, `show_chit_version`, `stop_daemon`, `clean_scenario`, `feedback_dir_for`, `collect_feedback`, `critique_generate`
- [ ] 1.2 Add 5-second `timeout` to `check_daemon_health` in `eval/lib.sh`
- [ ] 1.3 Add `HARNESS_MODE=1` support to all functions in `eval/lib.sh` — suppress banners, output structured data only
- [ ] 1.4 Rewrite `run.sh` to source `eval/lib.sh` and call the same functions — verify backward compatibility when no state file exists
- [ ] 1.5 Add state file helpers to `eval/lib.sh`: `state_read()` using `while IFS='=' read` (NOT `source`), `state_write()` using atomic `.tmp` + `mv`, `state_reset()`
- [ ] 1.6 Add PID lock file to `eval/lib.sh`: `lock_acquire()` writes PID to `eval/.harness.pid`, `lock_check()` verifies via `kill -0`, `lock_release()` removes on exit
- [ ] 1.7 Add stale `.harness-state.env.tmp` cleanup on startup (files older than 1 hour)
- [ ] 1.8 Add `eval/.harness-state.env`, `eval/.harness.pid`, `eval/*.tmp` to `.gitignore`
- [ ] 1.9 Add conditional precondition checks to `run.sh` — guard setup/collect/critique only when state file exists; backward compatible when absent
- [ ] 1.10 Add `aggregated-feedback.md` generation to `collect_feedback` in `eval/lib.sh`
- [ ] 1.11 Test: `run.sh setup cross-project` then `run.sh collect cross-project` works (no state file, no guards)
- [ ] 1.12 Test: state file exists, `run.sh collect cross-project` without prior setup prints error and exits non-zero
- [ ] 1.13 Test: `HARNESS_MODE=1 run.sh setup cross-project` suppresses banners
- [ ] 1.14 Test: PID lock prevents concurrent `run.sh` instances

## Phase 2: Harness interactive CLI

- [ ] 2.1 Create `eval/harness.sh` that sources `eval/lib.sh` and acquires PID lock on startup
- [ ] 2.2 Implement state machine transition table — valid states, valid transitions per state, precondition functions per transition
- [ ] 2.3 Implement `cmd_status()` — print current state, loop, scenario, available transitions, recommended next action
- [ ] 2.4 Implement `cmd_advance()` — validate current state, check preconditions, run transition action, write new state
- [ ] 2.5 Implement `advance_setup()` — clean temp dirs (preserve previous feedback/critic output), run setup function, verify daemon health, transition to `launching`
- [ ] 2.6 Implement `advance_collecting()` — re-check daemon health with timeout, run collect function, transition to `critiquing`
- [ ] 2.7 Implement `advance_critiquing()` — run critique function, verify critic prompt exists, transition to `analyzing`
- [ ] 2.8 Implement `advance_analyzing()` — check critic JSON output path, read it, evaluate exit criteria (P0+P1 count), print recommendation
- [ ] 2.9 Implement `advance_spec()` — transition to `specing` (prints reminder to create openspec change)
- [ ] 2.10 Implement `advance_exit()` — transition to `finished`
- [ ] 2.11 Implement `advance_pr()` — transition to `pr_ci` (prints reminder to commit and PR)
- [ ] 2.12 Implement loop logic: after pr_ci, read critic output for current loop, check exit criteria, increment loop or finish
- [ ] 2.13 Implement `cmd_scenario()` — set scenario name, validate against `eval/scenarios/*.md`
- [ ] 2.14 Implement `cmd_save_critic()` — read stdin, validate JSON (must have p0/p1/p2 arrays), write to iteration-indexed path
- [ ] 2.15 Implement `cmd_reset()` — reset state file to `initial` (preserves scenario name)
- [ ] 2.16 Implement `cmd_help()` — print usage with all commands
- [ ] 2.17 Integration test: full interactive loop for `cross-project` (manual)

## Phase 3: Auto mode + deterministic exit

- [ ] 3.1 Add `--auto` flag detection to `harness.sh`
- [ ] 3.2 In auto mode: all `cmd_*` functions output `KEY=VALUE` lines instead of human-readable text
- [ ] 3.3 In auto mode: `advance setup` outputs `TASK_PROMPT_ALPHA_FILE=<path>` and `TASK_PROMPT_BETA_FILE=<path>` — never inline content
- [ ] 3.4 Update critic prompt generation to include exact JSON schema and instruction to return JSON inline (not write to file)
- [ ] 3.5 Wire `save-critic` as the mechanism to capture critic output: auto mode prints "Paste critic output and pipe to: echo '<json>' | ./eval/harness.sh save-critic"
- [ ] 3.6 Handle malformed critic JSON — print parse error, fall back to manual confirmation
- [ ] 3.7 Integration test: auto mode with structured critic output, verify exit criteria decision

## Documentation

- [ ] 4.1 Rewrite `.opencode/skills/chit-eval/SKILL.md` — describe harness-driven workflow, state machine diagram, interactive usage with save-critic
- [ ] 4.2 Update `AGENTS.md` if needed
