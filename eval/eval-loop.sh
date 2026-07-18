#!/usr/bin/env bash
# tala eval loop — standalone orchestrator
# Each phase invokes a separate opencode agent or runs deterministic bash.
# The script owns ALL control flow; agents do exactly one narrow task each.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

# --- Config ---
SCENARIO="${1:-cross-project}"
MAX_LOOPS="${MAX_LOOPS:-5}"
AGENT_TIMEOUT="${AGENT_TIMEOUT:-1800}"       # 30 min per agent
OPENCODE_PORT="${OPENCODE_PORT:-0}"          # 0 = random
# Priority: env var > .eval.env > empty (opencode default)
env_model="${MODEL:-}"
env_variant="${VARIANT:-}"
config_read
MODEL="${env_model:-$MODEL}"
VARIANT="${env_variant:-$VARIANT}"
SERVER_LOG="$BASE_DIR/tmp/opencode-server.log"
HARNESS_LOG="$BASE_DIR/tmp/harness-output.log"
SERVER_URL=""

# --- Helpers ---

say() { echo "[eval-loop] $*"; }
die() { echo "[eval-loop] ERROR: $*" >&2; exit 1; }

# Portable timeout — use coreutils timeout if available, fall back to perl
_timeout() {
  local duration="$1"; shift
  if command -v gtimeout &>/dev/null; then
    gtimeout "$duration" "$@"
  elif command -v timeout &>/dev/null; then
    timeout "$duration" "$@"
  else
    perl -e '
      $SIG{ALRM} = sub { exit 124 };
      alarm shift;
      exec @ARGV;
    ' "$duration" "$@"
  fi
}

cleanup() {
  if [ -n "${SERVER_PID:-}" ]; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  stop_daemon
}
trap cleanup EXIT

# Run a harness command silently — suppress instructional output, log for debugging
# On failure, dump the last lines so the user sees what went wrong.
harness() {
  "$SCRIPT_DIR/harness.sh" "$@" >> "$HARNESS_LOG" 2>&1 || {
    local rc=$?
    echo "[eval-loop] HARNESS FAILED: $* (exit $rc)" >&2
    tail -5 "$HARNESS_LOG" | sed 's/^/  /' >&2
    return "$rc"
  }
}

start_server() {
  mkdir -p "$BASE_DIR/tmp"
  > "$SERVER_LOG"
  opencode serve --port "$OPENCODE_PORT" > "$SERVER_LOG" 2>&1 &
  SERVER_PID=$!
  say "Starting opencode server (PID $SERVER_PID)..."
  local i
  for i in $(seq 1 30); do
    SERVER_URL=$(sed -n 's/.*\(http:\/\/[0-9.]*:[0-9]*\).*/\1/p' "$SERVER_LOG" 2>/dev/null || true)
    if [ -n "$SERVER_URL" ]; then
      say "Server ready at $SERVER_URL"
      return 0
    fi
    sleep 1
  done
  die "Server failed to start within 30s. Log: $(cat "$SERVER_LOG")"
}

# Run an opencode agent with a given prompt, wait for completion, return stdout.
# Shows a live progress indicator with the agent's latest thinking line.
# Always returns 0 (prints warnings on failure) — never kills the parent script.
run_agent() {
  local prompt_file="$1" desc="$2" dir="${3:-}"
  local cmd=(opencode run --auto --attach "$SERVER_URL")
  if [ -n "$MODEL" ]; then cmd+=(--model "$MODEL"); fi
  if [ -n "$VARIANT" ]; then cmd+=(--variant "$VARIANT"); fi
  if [ -n "$dir" ]; then cmd+=(--dir "$dir"); fi
  local prompt
  prompt=$(cat "$prompt_file")
  say "Launching $desc..."
  local out_file err_file
  out_file=$(mktemp)
  err_file=$(mktemp)

  _timeout "$AGENT_TIMEOUT" "${cmd[@]}" "$prompt" > "$out_file" 2> "$err_file" &
  local agent_pid=$!

  local start_time
  start_time=$(date +%s)
  local spinner='-\|/'
  local sp_i=0
  local last_line=""
  local err_size=0

  while kill -0 "$agent_pid" 2>/dev/null; do
    local now elapsed
    now=$(date +%s)
    elapsed=$((now - start_time))

    # Check for new agent output
    if [ -f "$err_file" ]; then
      local new_size
      new_size=$(stat -f%z "$err_file" 2>/dev/null || stat -c%s "$err_file" 2>/dev/null || echo "0")
      if [ "$new_size" -gt "$err_size" ]; then
        err_size=$new_size
        local fresh
        local esc=$'\x1b'
        fresh=$(grep -v -E '(MaxListenersExceededWarning|overflowWarning|node:events|ExperimentalWarning|\(node:\d+|\(Bun:|Bun v|^build|^ar|^$)' "$err_file" 2>/dev/null | sed "s/${esc}\[[0-9;]*[a-zA-Z]//g" | tail -1 || true)
        if [ -n "$fresh" ]; then
          last_line="$fresh"
        fi
      fi
    fi

    local sp="${spinner:$sp_i:1}"
    sp_i=$(( (sp_i + 1) % 4 ))
    if [ -n "$last_line" ]; then
      # Truncate long lines to fit terminal
      local display="${last_line:0:80}"
      printf "\r  [%s] %s (%ds) %s" "$sp" "$desc" "$elapsed" "$display" >&2
    else
      printf "\r  [%s] %s (%ds)" "$sp" "$desc" "$elapsed" >&2
    fi
    sleep 2
  done

  # Clear the progress line
  printf "\r%80s\r" "" >&2

  wait "$agent_pid" || true
  local end_time elapsed
  end_time=$(date +%s)
  elapsed=$((end_time - start_time))
  say "$desc completed (${elapsed}s)"

  if [ -s "$err_file" ]; then
    local filtered
    filtered=$(grep -v -E '(MaxListenersExceededWarning|overflowWarning|node:events|ExperimentalWarning|\(node:\d+|\(Bun:|Bun v)' "$err_file" || true)
    if [ -n "$filtered" ]; then
      say "WARNING: $desc logged errors"
      echo "$filtered" | sed 's/^/  [stderr] /' >&2
      if [ -f "$SERVER_LOG" ]; then
        say "Server log (last 10 lines):"
        tail -10 "$SERVER_LOG" | sed 's/^/  /' >&2
      fi
    fi
  fi
  rm -f "$err_file"
  cat "$out_file" 2>/dev/null || true
  rm -f "$out_file"
}

extract_project_dir() {
  local prompt_file="$1"
  sed -n 's/.*cd \([^ ]*\).*/\1/p' "$prompt_file" 2>/dev/null | head -1 || true
}

report() {
  local phase="$1"
  local summary_file="${2:-}"
  echo ""
  echo "╔══════════════════════════════════════════╗"
  echo "║  Phase: $phase"
  if [ -n "$summary_file" ] && [ -f "$summary_file" ]; then
    echo "║"
    sed 's/^/║  /' "$summary_file"
  fi
  echo "╚══════════════════════════════════════════╝"
}

# --- Phase implementations ---

phase_setup() {
  harness scenario "$SCENARIO"
  harness advance setup
  local agent_count=0
  for f in "$AGENT_TASKS_DIR/$SCENARIO"/agent-*.md; do
    if [ -f "$f" ]; then agent_count=$((agent_count + 1)); fi
  done
  local ver
  ver=$("$TALA_BIN" --version 2>/dev/null || echo "unknown")
  local tmp
  tmp=$(mktemp)
  echo "scenario: $SCENARIO" > "$tmp"
  echo "agents: $agent_count" >> "$tmp"
  echo "tala: $ver" >> "$tmp"
  report "setup" "$tmp"
  rm -f "$tmp"
}

phase_launch() {
  local prompt_dir="$AGENT_TASKS_DIR/$SCENARIO"
  if [ ! -d "$prompt_dir" ]; then
    die "No prompt directory at $prompt_dir"
  fi

  local prompt_files=()
  for f in "$prompt_dir"/agent-*.md; do
    if [ -f "$f" ]; then prompt_files+=("$f"); fi
  done

  if [ ${#prompt_files[@]} -eq 0 ]; then
    say "WARNING: No agent prompt files found in $prompt_dir"
    return 0
  fi

  say "Launching ${#prompt_files[@]} sub-agent(s) in parallel..."

  local pids=()
  for prompt_file in "${prompt_files[@]}"; do
    local agent_name
    agent_name=$(basename "$prompt_file" .md)
    local project_dir
    project_dir=$(extract_project_dir "$prompt_file")
    (
      run_agent "$prompt_file" "Sub-agent $agent_name" "$project_dir" > /dev/null 2>&1 || true
    ) &
    pids+=($!)
  done

  local failed=0
  for pid in "${pids[@]}"; do
    if ! wait "$pid"; then
      failed=$((failed + 1))
    fi
  done

  local tmp
  tmp=$(mktemp)
  echo "launched: ${#prompt_files[@]}" > "$tmp"
  echo "succeeded: $(( ${#prompt_files[@]} - failed ))" >> "$tmp"
  echo "failed: $failed" >> "$tmp"
  report "launch" "$tmp"
  rm -f "$tmp"
}

phase_collect() {
  local feedback_dir
  feedback_dir=$(feedback_dir_for "$SCENARIO")
  local count=0
  if [ -d "$feedback_dir" ]; then
    for f in "$feedback_dir"/*.md; do
      if [ -f "$f" ]; then count=$((count + 1)); fi
    done
  fi
  harness advance collecting
  local tmp
  tmp=$(mktemp)
  echo "feedback files: $count" > "$tmp"
  report "collect" "$tmp"
  rm -f "$tmp"
}

validate_critic_json() {
  local json="$1"
  printf '%s\n' "$json" | python3 -c "
import json, sys
data = json.load(sys.stdin)
for k in ['p0', 'p1', 'p2']:
    assert k in data, f'Missing required key: {k}'
    assert isinstance(data[k], list), f'Key {k} must be an array, got {type(data[k]).__name__}'
" 2>&1
}

phase_critique() {
  harness advance critiquing

  local critic_prompt="$AGENT_TASKS_DIR/$SCENARIO/critic-prompt.md"
  if [ ! -f "$critic_prompt" ]; then
    die "Critic prompt not found at $critic_prompt"
  fi

  local max_retries=3
  local retry=0
  local critic_output=""
  local critic_json=""
  local validation_msg=""
  local ok=false

  while [ "$retry" -lt "$max_retries" ] && [ "$ok" = false ]; do
    if [ "$retry" -gt 0 ]; then
      local retry_prompt
      retry_prompt=$(mktemp)
      cat "$critic_prompt" > "$retry_prompt"
      cat >> "$retry_prompt" << PROMPT

## JSON Validation Failed

Your previous response did not produce valid JSON. Details:

$validation_msg

Please respond again with ONLY valid JSON matching the schema above. Do not include markdown code fences, explanatory text, or any formatting — just the raw JSON object. Make sure arrays use proper comma separators and there are no trailing commas.

PROMPT
      critic_output=$(run_agent "$retry_prompt" "Critic (retry $((retry + 1))/$max_retries)" "")
      rm -f "$retry_prompt"
    else
      say "Launching critic agent..."
      critic_output=$(run_agent "$critic_prompt" "Critic" "")
    fi

    critic_json=$(echo "$critic_output" | sed -n '/```json/,/```/p' | sed '1d;$d')
    if [ -z "$critic_json" ]; then
      critic_json=$(echo "$critic_output" | sed -n '/```/,/```/p' | sed '1d;$d')
    fi
    if [ -z "$critic_json" ]; then
      critic_json=$(echo "$critic_output" | grep -Eo '\{[^}]*"p0"[^}]*"p1"[^}]*"p2"[^}]*\}' | head -1)
    fi
    if [ -z "$critic_json" ]; then
      critic_json=$(echo "$critic_output" | grep -Eo '\{.*\}' | head -1)
    fi

    if [ -z "$critic_json" ]; then
      validation_msg="No JSON content found in response"
    else
      validation_msg=$(validate_critic_json "$critic_json")
    fi

    if [ -z "$validation_msg" ]; then
      ok=true
    else
      say "WARNING: Invalid JSON (attempt $((retry + 1))/$max_retries): ${validation_msg%%$'\n'*}"
    fi

    retry=$((retry + 1))
  done

  local tmp
  tmp=$(mktemp)

  if [ "$ok" = false ]; then
    say "WARNING: Could not extract valid JSON from critic output after $max_retries attempts. Using fallback."
    critic_json='{"p0":[],"p1":[],"p2":[],"summary":"extraction failed"}'
    echo "p0: 0" > "$tmp"
    echo "p1: 0" >> "$tmp"
    echo "p2: 0" >> "$tmp"
    echo "summary: extraction failed" >> "$tmp"
  else
    local p0 p1 p2 summary
    p0=$(echo "$critic_json" | jq -r '.p0 | length // 0' 2>/dev/null || echo "?")
    p1=$(echo "$critic_json" | jq -r '.p1 | length // 0' 2>/dev/null || echo "?")
    p2=$(echo "$critic_json" | jq -r '.p2 | length // 0' 2>/dev/null || echo "?")
    summary=$(echo "$critic_json" | jq -r '.summary // ""' 2>/dev/null || echo "")
    echo "p0: $p0" > "$tmp"
    echo "p1: $p1" >> "$tmp"
    echo "p2: $p2" >> "$tmp"
    [ -n "$summary" ] && echo "summary: $summary" >> "$tmp"
  fi

  echo "$critic_json" | "$SCRIPT_DIR/harness.sh" save-critic >> "$HARNESS_LOG" 2>&1
  report "critique" "$tmp"
  rm -f "$tmp"
}

phase_analyze() {
  state_read
  local loop_num="${LOOP:-0}"
  harness advance analyzing

  local loop_file="$AGENT_TASKS_DIR/$SCENARIO/critic-output-loop-${loop_num}.json"
  local tmp
  tmp=$(mktemp)

  if [ -f "$loop_file" ]; then
    local p0 p1 total
    p0=$(jq '.p0 | length' "$loop_file")
    p1=$(jq '.p1 | length' "$loop_file")
    total=$((p0 + p1))
    echo "p0: $p0" > "$tmp"
    echo "p1: $p1" >> "$tmp"
    echo "total: $total" >> "$tmp"
    if [ "$total" -eq 0 ]; then
      EXIT_CRITERIA_MET=true
      echo "exit: criteria met" >> "$tmp"
    else
      EXIT_CRITERIA_MET=false
      echo "exit: issues remain" >> "$tmp"
    fi
  else
    echo "error: no critic output found" > "$tmp"
    EXIT_CRITERIA_MET=true
  fi

  report "analyze" "$tmp"
  rm -f "$tmp"
}

phase_implement() {
  state_read
  local loop_num="${LOOP:-0}"

  local p0 p1 p2 total
  p0=$(jq '(.p0 | length)' "$AGENT_TASKS_DIR/$SCENARIO/critic-output-loop-${loop_num}.json" 2>/dev/null || echo "?")
  p1=$(jq '(.p1 | length)' "$AGENT_TASKS_DIR/$SCENARIO/critic-output-loop-${loop_num}.json" 2>/dev/null || echo "?")
  p2=$(jq '(.p2 | length)' "$AGENT_TASKS_DIR/$SCENARIO/critic-output-loop-${loop_num}.json" 2>/dev/null || echo "?")
  if [ "$p0" = "?" ] || [ "$p1" = "?" ] || [ "$p2" = "?" ]; then
    total="?"
  else
    total=$((p0 + p1 + p2))
  fi

  local implement_dir="$BASE_DIR/tmp/implement-${loop_num}"
  mkdir -p "$implement_dir"
  local triage_file="$implement_dir/triage.json"
  local summary_file="$BASE_DIR/tmp/implement-summary-${loop_num}.json"

  # --- Agent 1: Triage + openspec artifacts ---
  local spec_prompt="$implement_dir/prompt-spec.md"
  cat > "$spec_prompt" << PROMPT
# Eval Fix Loop $loop_num — Triage & Spec

You are planning fixes for issues found during the tala eval loop.

## Context

The eval scenario "$SCENARIO" has identified $total item(s) across all priorities:
- P0 (must fix): $p0
- P1 (should fix): $p1
- P2 (nice to have): $p2

Read the full critic output at: $AGENT_TASKS_DIR/$SCENARIO/critic-output-loop-${loop_num}.json

## Your Tasks

1. **Triage all items.** Read the critic output. For each item, decide: fix, defer, or exclude with rationale. You have full remit to exclude any item if it's not actionable, already fixed, or out of scope.

2. **Propose a change name.** Choose a short descriptive kebab-case name (e.g. "fix-csv-parsing"). Do not include the loop number.

3. Create the openspec change and all artifacts:
   - \`openspec new change <name>\`
   - Run \`openspec instructions proposal --change <name>\` and write the proposal file
   - Continue creating each artifact (specs, design, tasks) using \`openspec instructions\`
   - If openspec tells you to STOP, IGNORE that — continue until all artifacts exist

4. **Red-team the spec** — review for gaps and flaws.

5. Write your triage plan to: $triage_file
   Format:
   \`\`\`json
   {"change_name":"<name>","issues_fixed":[{"priority":"P1","description":"..."}],"excluded_items":[{"description":"...","rationale":"..."}],"red_team_findings":["..."]}
   \`\`\`
PROMPT

  say "Launching Triage & Spec agent..."
  run_agent "$spec_prompt" "Triage & Spec" "$SCRIPT_DIR/.."
  rm -f "$spec_prompt"

  local change_name="fix-loop-${loop_num}"
  if [ -f "$triage_file" ]; then
    change_name=$(jq -r '.change_name // empty' "$triage_file" 2>/dev/null || echo "fix-loop-${loop_num}")
    say "Triage complete. Change: $change_name"
  else
    say "WARNING: Triage agent did not write triage file. Falling back to '$change_name'."
  fi

  # --- Agent 2: Implement tasks from the spec ---
  local impl_prompt="$implement_dir/prompt-implement.md"
  cat > "$impl_prompt" << PROMPT
# Eval Fix Loop $loop_num — Implementation

You are implementing fixes for the openspec change "$change_name".

## Your Tasks

1. Read the task list from the openspec artifacts:
   - Change directory: $SCRIPT_DIR/../openspec/changes/$change_name/
   - Tasks file: tasks.md

2. Implement all tasks from the tasks.md file

3. When done, commit all changes:
   - \`git add -A\`
   - \`git commit -m "$change_name: implement eval fix loop $loop_num"\`

4. Write a JSON summary of what you did to: $summary_file
   Include fields: change_name, commits (array), files_changed (array), issues_fixed (array), excluded_items (array), red_team_findings (array)
   Example:
   \`\`\`json
   {"change_name":"$change_name","commits":["abc123"],"files_changed":["src/main.rs"],"issues_fixed":["fixed bug X"],"excluded_items":[{"description":"Y","rationale":"deferred"}],"red_team_findings":["found edge case Z"]}
   \`\`\`
PROMPT

  say "Launching Implementation agent..."
  run_agent "$impl_prompt" "Implementation" "$SCRIPT_DIR/.."
  rm -f "$impl_prompt"

  # --- Read results ---
  if [ -f "$summary_file" ]; then
    change_name=$(jq -r '.change_name // empty' "$summary_file" 2>/dev/null || echo "$change_name")
    report "implement" "$summary_file"
    rm -f "$summary_file"
  else
    local tmp
    tmp=$(mktemp)
    echo "change: $change_name" > "$tmp"
    echo "summary: agent did not write summary file" >> "$tmp"
    report "implement" "$tmp"
    rm -f "$tmp"
    say "WARNING: Implementation agent did not write summary file."
  fi

  # store change_name for subsequent phases
  echo "$change_name" > "$BASE_DIR/tmp/change-name-${loop_num}.txt"
  rm -rf "$implement_dir"
}

wait_for_pr_merge() {
  local pr_url="$1" branch_name="$2" max_wait="${3:-900}"
  say "Waiting for CI checks on $branch_name (timeout: ${max_wait}s)..."
  if ! gh pr checks "$pr_url" --watch --interval 30 -t "$max_wait" 2>/dev/null; then
    say "ERROR: CI checks failed for $branch_name"
    gh pr checks "$pr_url" --fail-fast 2>/dev/null || true
    return 1
  fi
  say "CI checks passed for $branch_name"
  local merge_state
  merge_state=$(gh pr view "$pr_url" --json mergeStateStatus --jq '.mergeStateStatus' 2>/dev/null || echo "unknown")
  if [ "$merge_state" = "MERGEABLE" ] || [ "$merge_state" = "CLEAN" ]; then
    say "PR is mergeable — merging now..."
    gh pr merge --squash "$pr_url" 2>/dev/null || true
    local i
    for i in $(seq 1 30); do
      local pr_state
      pr_state=$(gh pr view "$pr_url" --json state --jq '.state' 2>/dev/null || echo "")
      if [ "$pr_state" = "MERGED" ]; then
        say "PR merged successfully"
        return 0
      fi
      sleep 5
    done
    say "WARNING: Timed out waiting for merge to complete"
    return 1
  elif [ "$merge_state" = "DIRTY" ] || [ "$merge_state" = "BLOCKED" ] || [ "$merge_state" = "GATING" ]; then
    say "WARNING: PR is not mergeable (state=$merge_state). Attempting auto-merge..."
    gh pr merge --auto --squash "$pr_url" 2>/dev/null || true
    local i
    for i in $(seq 1 60); do
      local pr_state
      pr_state=$(gh pr view "$pr_url" --json state --jq '.state' 2>/dev/null || echo "")
      if [ "$pr_state" = "MERGED" ]; then
        say "PR merged successfully"
        return 0
      fi
      sleep 10
    done
    say "WARNING: Timed out waiting for PR merge"
    return 1
  else
    say "WARNING: Unknown merge state '$merge_state'"
    gh pr merge --auto --squash "$pr_url" 2>/dev/null || true
    return 1
  fi
}

phase_finalize() {
  state_read
  local loop_num="${LOOP:-0}"

  local change_name
  if [ -f "$BASE_DIR/tmp/change-name-${loop_num}.txt" ]; then
    change_name=$(cat "$BASE_DIR/tmp/change-name-${loop_num}.txt")
  else
    change_name="fix-loop-${loop_num}"
    say "WARNING: no stored change_name, falling back to '$change_name'"
  fi

  local branch_name="${change_name}-${loop_num}"

  cd "$SCRIPT_DIR/.."

  local tmp
  tmp=$(mktemp)

  local current_branch
  current_branch=$(git rev-parse --abbrev-ref HEAD)

  if [ "$current_branch" != "$branch_name" ]; then
    if git show-ref --verify --quiet "refs/heads/$branch_name"; then
      git checkout "$branch_name"
    else
      git checkout -b "$branch_name"
    fi
  fi

  local pr_url=""
  git push origin "$branch_name" 2>/dev/null || {
    say "WARNING: Push failed. Retrying with --force..."
    git push --force origin "$branch_name" 2>/dev/null || true
  }

  pr_url=$(gh pr create --fill --base main 2>/dev/null || true)
  if [ -z "$pr_url" ]; then
    pr_url=$(gh pr list --head "$branch_name" --json url --jq '.[0].url' 2>/dev/null || true)
  fi

  local merge_ok=false
  if [ -n "$pr_url" ]; then
    if wait_for_pr_merge "$pr_url" "$branch_name"; then
      merge_ok=true
    fi
  fi

  echo "branch: $branch_name" > "$tmp"
  [ -n "$pr_url" ] && echo "pr: $pr_url" >> "$tmp"
  echo "merged: $merge_ok" >> "$tmp"
  report "finalize" "$tmp"
  rm -f "$tmp"

  if [ "$merge_ok" != "true" ]; then
    say "ERROR: PR was not merged successfully for $branch_name. Stopping loop."
    exit 1
  fi
}

phase_exit() {
  harness advance exit
  local tmp
  tmp=$(mktemp)
  echo "state: finished" > "$tmp"
  report "exit" "$tmp"
  rm -f "$tmp"
  say "Eval complete."
}

print_loop_header() {
  local loop_num="$1" max="$2"
  echo ""
  echo "┌─────────────────────────────────────────────┐"
  echo "│  Loop $loop_num / $max"
  echo "└─────────────────────────────────────────────┘"
}

# --- Main ---

main() {
  if [ ! -f "$SCENARIOS_DIR/$SCENARIO.md" ]; then
    echo "Available scenarios:"
    for f in "$SCENARIOS_DIR"/*.md; do
      echo "  $(basename "$f" .md)"
    done
    die "Unknown scenario: $SCENARIO"
  fi

  echo ""
  echo "┌─────────────────────────────────────────────┐"
  echo "│  tala eval loop"
  echo "│  scenario: $SCENARIO"
  echo "│  max loops: $MAX_LOOPS"
  echo "│  agent timeout: ${AGENT_TIMEOUT}s"
  echo "│  model: ${MODEL:-<default>}"
  echo "│  variant: ${VARIANT:-<default>}"
  echo "└─────────────────────────────────────────────┘"

  start_server

  local loop_num=0
  while [ "$loop_num" -lt "$MAX_LOOPS" ]; do
    state_read
    print_loop_header "${LOOP:-0}" "$MAX_LOOPS"

    phase_setup
    phase_launch
    phase_collect
    phase_critique
    phase_analyze

    if [ "${EXIT_CRITERIA_MET:-false}" = "true" ]; then
      phase_exit
      return 0
    fi

    harness advance spec || true
    phase_implement
    phase_finalize
    harness advance pr || true

    loop_num=$((loop_num + 1))
  done

  say "Max loops ($MAX_LOOPS) reached. Exiting."
  harness advance exit || true
}

main
