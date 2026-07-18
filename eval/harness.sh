#!/usr/bin/env bash
# chit eval harness — deterministic state machine
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib.sh"
source "$SCRIPT_DIR/run.sh"

AUTO_MODE=false
if [ "${1:-}" = "--auto" ]; then
  AUTO_MODE=true
  HARNESS_MODE=1
  shift
fi

cleanup_stale_tmp
lock_acquire
trap lock_release EXIT

# In auto mode, suppress human-readable output.
# status_line always prints for machine consumption.
say() {
  if [ "$AUTO_MODE" != true ]; then
    echo "$@"
  fi
}

status_line() {
  local key="$1" value="$2"
  if [ "$AUTO_MODE" = true ]; then
    echo "${key}=${value}"
  fi
}

# --- Transition table ---

transition_allowed() {
  local from="$1" target="$2"
  case "$from|$target" in
    "initial|setup"|"pr_ci|setup") return 0 ;;
    "launching|collecting") return 0 ;;
    "collecting|critiquing") return 0 ;;
    "critiquing|analyzing"|"analyzing|analyzing") return 0 ;;
    "analyzing|spec") return 0 ;;
    "analyzing|exit") return 0 ;;
    "specing|pr") return 0 ;;
    "pr_ci|exit") return 0 ;;
    *) return 1 ;;
  esac
}

# --- Preconditions ---

precond_requires() {
  local msg="$1"
  if [ "$AUTO_MODE" = true ]; then
    echo "PRECONDITION_FAILED=${msg}"
  fi
  echo "Error: ${msg}" >&2
  echo "Current state: ${STATE}" >&2
  exit 1
}

precond_setup() {
  if [ -z "${SCENARIO:-}" ]; then
    precond_requires "No scenario set. Run 'harness.sh scenario <name>' first."
  fi
}

precond_collecting() {
  local fb_dir
  fb_dir=$(feedback_dir_for "$SCENARIO")
  if [ ! -d "$fb_dir" ]; then
    echo "Warning: No feedback directory found at $fb_dir" >&2
    echo "Sub-agents may not have written feedback yet." >&2
  fi
  # Check daemon health (best-effort warning)
  if [ -f "$BASE_DIR/tmp/daemon.pid" ]; then
    local dp_pid
    dp_pid=$(cat "$BASE_DIR/tmp/daemon.pid")
    if ! kill -0 "$dp_pid" 2>/dev/null; then
      echo "Warning: Daemon is not running. Feedback may be incomplete." >&2
    fi
  fi
}

precond_critiquing() {
  local agg="$AGENT_TASKS_DIR/$SCENARIO/aggregated-feedback.md"
  if [ ! -f "$agg" ]; then
    precond_requires "No aggregated feedback found at $agg. Run 'advance collecting' first."
  fi
}

precond_analyzing() {
  local cp="$AGENT_TASKS_DIR/$SCENARIO/critic-prompt.md"
  if [ ! -f "$cp" ]; then
    precond_requires "No critic prompt found at $cp. Run 'advance critiquing' first."
  fi
}

precond_spec() {
  local loop_file="$AGENT_TASKS_DIR/$SCENARIO/critic-output-loop-${LOOP}.json"
  if [ ! -f "$loop_file" ]; then
    echo "Warning: No critic output found at $loop_file." >&2
    echo "Use 'save-critic' to save the critic's JSON response first." >&2
    echo "Proceeding anyway (manual confirmation assumed)." >&2
    return 0
  fi
  local total
  total=$(python3 -c "
import json,sys
d=json.load(open('$loop_file'))
print(len(d.get('p0',[])) + len(d.get('p1',[])))
" 2>/dev/null || echo "0")
  if [ "$total" -eq 0 ] 2>/dev/null; then
    echo "Warning: Critic found no P0 or P1 items (P0+P1=0)." >&2
    echo "Consider 'advance exit' instead." >&2
  fi
}

precond_pr() {
  :
}

precond_exit_criteria() {
  if [ "$LOOP" -ge "$MAX_LOOPS" ] 2>/dev/null; then
    say "Max loops ($MAX_LOOPS) reached."
    return 0
  fi
  local loop_file="$AGENT_TASKS_DIR/$SCENARIO/critic-output-loop-${LOOP}.json"
  if [ -f "$loop_file" ]; then
    local total
    total=$(python3 -c "
import json,sys
d=json.load(open('$loop_file'))
print(len(d.get('p0',[])) + len(d.get('p1',[])))
" 2>/dev/null || echo "1")
    if [ "$total" -eq 0 ] 2>/dev/null; then
      return 0
    fi
    echo "Warning: $total material issue(s) remain (P0+P1 > 0)." >&2
    echo "Use 'advance spec' to address them." >&2
    return 1
  fi
  return 1
}

# --- Transition actions ---

advance_setup() {
  local scenario="${SCENARIO}"
  mkdir -p "$BASE_DIR/tmp"
  clean_scenario "$scenario"
  local setup_func="setup_${scenario//-/_}"
  if declare -f "$setup_func" > /dev/null 2>&1; then
    if [ "$AUTO_MODE" = true ]; then
      "$setup_func" > /dev/null
    else
      "$setup_func"
    fi
  else
    echo "Error: Unknown scenario '$scenario'. No setup function found." >&2
    exit 1
  fi
  STATE=launching
  state_write
  say ""
  say "Sub-agents ready."
  say "Copy task prompts from: $AGENT_TASKS_DIR/$scenario/"
  say "After sub-agents complete, run: ./eval/harness.sh advance collecting"
  status_line "STATE" "$STATE"
  status_line "LOOP" "${LOOP:-0}"
  # Output file paths for each agent prompt
  local prompt_dir="$AGENT_TASKS_DIR/$scenario"
  if [ -d "$prompt_dir" ]; then
    for f in "$prompt_dir"/agent-*.md; do
      if [ -f "$f" ]; then
        local agent_name
        agent_name=$(basename "$f" .md | tr '[:lower:]' '[:upper:]' | sed 's/AGENT-//')
        status_line "TASK_PROMPT_${agent_name}_FILE" "$f"
      fi
    done
  fi
  status_line "RECOMMENDED_NEXT" "collecting"
}

advance_collecting() {
  precond_collecting
  local collect_func="collect_${SCENARIO//-/_}"
  if declare -f "$collect_func" > /dev/null 2>&1; then
    if [ "$AUTO_MODE" = true ]; then
      "$collect_func" > /dev/null
    else
      "$collect_func"
    fi
  else
    if [ "$AUTO_MODE" = true ]; then
      collect_feedback "$SCENARIO" > /dev/null
    else
      collect_feedback "$SCENARIO"
    fi
  fi
  STATE=collecting
  state_write
  say ""
  say "Next: ./eval/harness.sh advance critiquing"
  status_line "STATE" "$STATE"
  status_line "LOOP" "${LOOP:-0}"
  status_line "RECOMMENDED_NEXT" "critiquing"
}

advance_critiquing() {
  precond_critiquing
  local critique_func="critique_${SCENARIO//-/_}"
  if declare -f "$critique_func" > /dev/null 2>&1; then
    if [ "$AUTO_MODE" = true ]; then
      "$critique_func" > /dev/null
    else
      "$critique_func"
    fi
  else
    echo "Error: No critique function for scenario '$SCENARIO'." >&2
    exit 1
  fi
  STATE=critiquing
  state_write
  local cp="$AGENT_TASKS_DIR/$SCENARIO/critic-prompt.md"
  say ""
  say "Critic prompt generated."
  say "Copy the prompt from: $cp"
  say "Paste it into a Task tool call for the critic sub-agent."
  say "After the critic responds, save their JSON output:"
  say "  echo '<critic-json>' | ./eval/harness.sh save-critic"
  say "Then run: ./eval/harness.sh advance analyzing"
  status_line "STATE" "$STATE"
  status_line "LOOP" "${LOOP:-0}"
  status_line "CRITIC_PROMPT_FILE" "$cp"
  status_line "SAVE_CRITIC_CMD" "echo '<critic-json>' | ./eval/harness.sh save-critic"
  status_line "RECOMMENDED_NEXT" "analyzing"
}

advance_analyzing() {
  precond_analyzing
  STATE=analyzing
  state_write
  local loop_file="$AGENT_TASKS_DIR/$SCENARIO/critic-output-loop-${LOOP}.json"
  if [ -f "$loop_file" ]; then
    local p0_count p1_count p2_count summary
    p0_count=$(python3 -c "import json,sys; d=json.load(open('$loop_file')); print(len(d.get('p0',[])))" 2>/dev/null || echo "?")
    p1_count=$(python3 -c "import json,sys; d=json.load(open('$loop_file')); print(len(d.get('p1',[])))" 2>/dev/null || echo "?")
    p2_count=$(python3 -c "import json,sys; d=json.load(open('$loop_file')); print(len(d.get('p2',[])))" 2>/dev/null || echo "?")
    summary=$(python3 -c "import json,sys; d=json.load(open('$loop_file')); print(d.get('summary',''))" 2>/dev/null || echo "")
    say "=== Critic Report (Loop $LOOP) ==="
    say "P0 items: $p0_count"
    say "P1 items: $p1_count"
    say "P2 items: $p2_count"
    if [ -n "$summary" ]; then
      say "Summary: $summary"
    fi
    say ""
    status_line "P0_COUNT" "$p0_count"
    status_line "P1_COUNT" "$p1_count"
    status_line "P2_COUNT" "$p2_count"
    [ -n "$summary" ] && status_line "CRITIC_SUMMARY" "$summary"
    if [ "${p0_count}" != "?" ] && [ "$p0_count" -eq 0 ] && [ "$p1_count" -eq 0 ]; then
      say "No P0 or P1 items. Exit criteria met."
      say "Run: ./eval/harness.sh advance exit"
      status_line "EXIT_CRITERIA_MET" "true"
      status_line "RECOMMENDED_NEXT" "exit"
    else
      say "${p0_count} P0 + ${p1_count} P1 = $((p0_count + p1_count)) material issue(s) found."
      say "To fix them: ./eval/harness.sh advance spec"
      say "To exit anyway: ./eval/harness.sh advance exit"
      status_line "EXIT_CRITERIA_MET" "false"
      status_line "MATERIAL_ISSUES" "$((p0_count + p1_count))"
      status_line "RECOMMENDED_NEXT" "spec"
    fi
  else
    say "No critic output found for loop $LOOP."
    say "Use 'save-critic' to save the critic's JSON response."
    say "File expected at: $loop_file"
    say "Then run 'advance analyzing' again."
    status_line "CRITIC_FILE" "$loop_file"
    status_line "CRITIC_MISSING" "true"
  fi
  status_line "STATE" "$STATE"
  status_line "LOOP" "${LOOP:-0}"
}

advance_spec() {
  precond_spec
  STATE=specing
  state_write
  say "=== Spec Phase ==="
  say "1. Create an openspec change:"
  say "     openspec new change \"<kebab-case-name>\""
  say "   Work through proposal → specs → design → tasks."
  say ""
  say "2. Red-team the spec through a sub-agent before implementing:"
  say "     task description=\"Red-team <change>\" subagent_type=\"general\" prompt=\"..."
  say "   Fix any issues found by the red-team before proceeding."
  say ""
  say "3. Implement the tasks."
  say "4. After implementation, run: ./eval/harness.sh advance pr"
  status_line "STATE" "$STATE"
  status_line "LOOP" "${LOOP:-0}"
  status_line "RECOMMENDED_NEXT" "pr"
}

advance_pr() {
  precond_pr
  STATE=pr_ci
  state_write
  say "=== PR & CI Phase ==="
  say "1. Commit your changes"
  say "2. Push and create a PR"
  say "3. Wait for CI to pass"
  say "4. Merge"
  say ""
  LOOP=$((LOOP + 1))
  state_write
  say "Loop $LOOP complete."
  status_line "STATE" "$STATE"
  status_line "LOOP" "${LOOP:-0}"
  if [ "$LOOP" -ge "$MAX_LOOPS" ] 2>/dev/null; then
    say "Max loops ($MAX_LOOPS) reached."
    say "Run: ./eval/harness.sh advance exit"
    status_line "MAX_LOOPS_REACHED" "true"
    status_line "RECOMMENDED_NEXT" "exit"
  else
    say "Ready for next iteration."
    say "Run: ./eval/harness.sh advance setup"
    say "Or to stop: ./eval/harness.sh advance exit"
    status_line "MAX_LOOPS_REACHED" "false"
    status_line "RECOMMENDED_NEXT" "setup or exit"
  fi
}

advance_exit() {
  if ! precond_exit_criteria; then
    echo "Use '--force' to exit anyway." >&2
    exit 1
  fi
  STATE=finished
  state_write
  say "=== Eval Complete ==="
  say "Ran $LOOP loop(s) for scenario '$SCENARIO'."
  say "Results in: $AGENT_TASKS_DIR/$SCENARIO/"
  say ""
  say "To start fresh: ./eval/harness.sh reset"
  status_line "STATE" "$STATE"
  status_line "LOOP" "${LOOP:-0}"
  status_line "SCENARIO" "${SCENARIO:-}"
  status_line "RESULTS_DIR" "$AGENT_TASKS_DIR/${SCENARIO:-}"
  status_line "RECOMMENDED_NEXT" "reset"
}

# --- Checking state transitions ---

cmd_advance() {
  local target="$1"
  state_read
  if [ "$STATE" = "finished" ]; then
    echo "Error: Eval is finished. Run 'reset' to start over." >&2
    exit 1
  fi
  if ! transition_allowed "$STATE" "$target"; then
    echo "Error: Cannot advance from '$STATE' to '$target'." >&2
    echo "Allowed targets from '$STATE':" >&2
    for t in setup collecting critiquing analyzing spec exit pr; do
      if transition_allowed "$STATE" "$t"; then
        echo "  $t" >&2
      fi
    done
    exit 1
  fi
  status_line "TRANSITION" "${STATE}→${target}"
  "advance_${target}"
}

cmd_status() {
  state_read
  say "State:      $STATE"
  say "Scenario:   ${SCENARIO:-<not set>}"
  say "Loop:       ${LOOP:-0}/${MAX_LOOPS:-5}"
  say ""
  if [ "$STATE" = "finished" ]; then
    say "Eval is complete. Run 'reset' to start over."
    status_line "AVAILABLE" "reset"
    status_line "RECOMMENDED_NEXT" "reset"
    return
  fi
  local targets=""
  for t in setup collecting critiquing analyzing spec exit pr; do
    if transition_allowed "$STATE" "$t"; then
      targets="$targets $t"
    fi
  done
  say "Available: ${targets# }"
  say ""
  say "Recommended next:"
  case "$STATE" in
    initial)
      say "  ./eval/harness.sh scenario <name>   (if not set)"
      say "  ./eval/harness.sh advance setup"
      status_line "RECOMMENDED_NEXT" "setup"
      ;;
    launching)
      say "  Launch sub-agents via Task tool, then:"
      say "  ./eval/harness.sh advance collecting"
      status_line "RECOMMENDED_NEXT" "collecting"
      ;;
    collecting)
      say "  ./eval/harness.sh advance critiquing"
      status_line "RECOMMENDED_NEXT" "critiquing"
      ;;
    critiquing)
      say "  Copy critic prompt, launch critic via Task tool,"
      say "  save result with 'save-critic', then:"
      say "  ./eval/harness.sh advance analyzing"
      status_line "RECOMMENDED_NEXT" "analyzing"
      ;;
    analyzing)
      local loop_file="$AGENT_TASKS_DIR/$SCENARIO/critic-output-loop-${LOOP}.json"
      if [ -f "$loop_file" ]; then
        local p0_count p1_count p2_count summary
        p0_count=$(python3 -c "import json,sys; d=json.load(open('$loop_file')); print(len(d.get('p0',[])))" 2>/dev/null || echo "?")
        p1_count=$(python3 -c "import json,sys; d=json.load(open('$loop_file')); print(len(d.get('p1',[])))" 2>/dev/null || echo "?")
        p2_count=$(python3 -c "import json,sys; d=json.load(open('$loop_file')); print(len(d.get('p2',[])))" 2>/dev/null || echo "?")
        summary=$(python3 -c "import json,sys; d=json.load(open('$loop_file')); print(d.get('summary',''))" 2>/dev/null || echo "")
        say "  P0: $p0_count  P1: $p1_count  P2: $p2_count"
        [ -n "$summary" ] && echo "  Summary: $summary"
      else
        say "  No critic output saved yet."
        say "  Use: echo '<critic-json>' | ./eval/harness.sh save-critic"
      fi
      say ""
      say "  ./eval/harness.sh advance spec   (to fix issues)"
      say "  ./eval/harness.sh advance exit   (if no issues)"
      status_line "RECOMMENDED_NEXT" "spec or exit"
      ;;
    specing)
      say "  1. Create openspec change, red-team via sub-agent, implement"
      say "  2. ./eval/harness.sh advance pr   (after implementation)"
      status_line "RECOMMENDED_NEXT" "pr"
      ;;
    pr_ci)
      say "  Merge PR, then:"
      say "  ./eval/harness.sh advance setup   (next iteration)"
      say "  ./eval/harness.sh advance exit    (stop here)"
      status_line "RECOMMENDED_NEXT" "setup or exit"
      ;;
  esac
  status_line "STATE" "$STATE"
  status_line "LOOP" "${LOOP:-0}"
  status_line "SCENARIO" "${SCENARIO:-}"
  status_line "AVAILABLE" "${targets# }"
}

cmd_scenario() {
  local name="${1:-}"
  if [ -z "$name" ]; then
    say "Available scenarios:"
    for f in "$SCENARIOS_DIR"/*.md; do
      local base
      base=$(basename "$f" .md)
      say "  $base"
    done
    status_line "SCENARIOS" "$(ls "$SCENARIOS_DIR"/*.md 2>/dev/null | xargs -n1 basename -s .md | tr '\n' ' ')"
    return
  fi
  if [ ! -f "$SCENARIOS_DIR/$name.md" ]; then
    echo "Error: Unknown scenario '$name'." >&2
    say "Available:"
    for f in "$SCENARIOS_DIR"/*.md; do
      local base
      base=$(basename "$f" .md)
      echo "  $base" >&2
    done
    status_line "ERROR" "unknown scenario: $name"
    exit 1
  fi
  SCENARIO="$name"
  state_write
  say "Scenario set to '$name'."
  say "Run: ./eval/harness.sh advance setup"
  status_line "SCENARIO" "$name"
}

cmd_save_critic() {
  state_read
  if [ -z "$SCENARIO" ]; then
    echo "Error: No scenario set." >&2
    exit 1
  fi
  mkdir -p "$AGENT_TASKS_DIR/$SCENARIO"
  local loop_file="$AGENT_TASKS_DIR/$SCENARIO/critic-output-loop-${LOOP}.json"
  local tmp_file="${loop_file}.tmp"
  # Read stdin, validate JSON
  if ! python3 -c "
import json,sys
data = json.load(sys.stdin)
required = ['p0','p1','p2']
for k in required:
    if k not in data:
        print(f'Missing required key: {k}', file=sys.stderr)
        sys.exit(1)
    if not isinstance(data[k], list):
        print(f'Key {k} must be an array', file=sys.stderr)
        sys.exit(1)
print(json.dumps(data, indent=2))
" > "$tmp_file" 2>"${tmp_file}.err"; then
    echo "Error: Invalid critic JSON." >&2
    cat "${tmp_file}.err" >&2
    rm -f "$tmp_file" "${tmp_file}.err"
    exit 1
  fi
  rm -f "${tmp_file}.err"
  mv "$tmp_file" "$loop_file"
  say "Critic output saved to $loop_file"
  say "Run: ./eval/harness.sh advance analyzing"
  status_line "CRITIC_SAVED" "$loop_file"
}

cmd_reset() {
  state_read
  local old_scenario="$SCENARIO"
  state_reset
  say "State reset to initial."
  if [ -n "$old_scenario" ]; then
    SCENARIO="$old_scenario"
    state_write
    say "Preserved scenario: $old_scenario"
  fi
  status_line "STATE" "initial"
}

cmd_help() {
  cat << HELP
chit eval harness — interactive state machine

Commands:
  status                  Show current state, loop, available transitions
  advance <target>        Advance to next state (guarded by preconditions)
  scenario [<name>]       Set or list eval scenarios
  save-critic             Save critic JSON output from stdin
  reset                   Reset state machine to initial
  help                    Print this help

State machine:
  initial ──setup──→ launching ──collecting──→ collecting
  collecting ──critiquing──→ critiquing ──analyzing──→ analyzing
  analyzing ──spec──→ specing ──pr──→ pr_ci ──setup/exit──→ ...
  analyzing ──exit──→ finished

Targets per state:
  initial/pr_ci:  setup
  launching:      collecting
  collecting:     critiquing
  critiquing:     analyzing
  analyzing:      spec, exit
  specing:        pr
  pr_ci:          setup, exit
HELP
}

# --- Main dispatch ---

case "${1:-help}" in
  status)
    cmd_status
    ;;
  advance)
    if [ -z "${2:-}" ]; then
      echo "Usage: $0 advance <target>" >&2
      echo "Targets: setup collecting critiquing analyzing spec exit pr" >&2
      exit 1
    fi
    cmd_advance "$2"
    ;;
  scenario)
    cmd_scenario "${2:-}"
    ;;
  save-critic)
    cmd_save_critic
    ;;
  reset)
    cmd_reset
    ;;
  help|--help|-h)
    cmd_help
    ;;
  *)
    echo "Usage: $0 {status|advance|scenario|save-critic|reset|help}" >&2
    exit 1
    ;;
esac
