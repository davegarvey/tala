#!/usr/bin/env bash
# chit eval shared library
# Source this from run.sh or harness.sh.
# Set HARNESS_MODE=1 to suppress human-friendly banners.

set -euo pipefail

# --- Paths ---
CHIT_BIN="${CHIT_BIN:-$(dirname "${BASH_SOURCE[0]}")/../target/release/chit}"
BASE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCENARIOS_DIR="$BASE_DIR/scenarios"
AGENT_TASKS_DIR="$BASE_DIR/agent-tasks"
STATE_FILE="$BASE_DIR/.harness-state.env"
PID_FILE="$BASE_DIR/.harness.pid"

if [ ! -f "$CHIT_BIN" ]; then
  CHIT_BIN="$(dirname "${BASH_SOURCE[0]}")/../target/debug/chit"
fi
if [ ! -f "$CHIT_BIN" ]; then
  echo "Error: chit binary not found. Build with: cargo build --release"
  exit 1
fi

# --- Output helpers ---

hdr() {
  if [ "${HARNESS_MODE:-0}" != "1" ]; then
    echo "==========================================="
    echo "  $*"
    echo "==========================================="
  fi
}

msg() {
  if [ "${HARNESS_MODE:-0}" != "1" ]; then
    echo "$@"
  fi
}

# --- State file (env-var format, line-by-line parse, NOT sourced) ---

STATE_KEYS=(HARNESS_VERSION STATE SCENARIO LOOP MAX_LOOPS HARNESS_PID)

state_read() {
  HARNESS_VERSION=1
  STATE=initial
  SCENARIO=
  LOOP=0
  MAX_LOOPS=5
  HARNESS_PID=
  if [ -f "$STATE_FILE" ]; then
    while IFS='=' read -r key value; do
      case "$key" in
        HARNESS_VERSION) HARNESS_VERSION="$value" ;;
        STATE) STATE="$value" ;;
        SCENARIO) SCENARIO="$value" ;;
        LOOP) LOOP="$value" ;;
        MAX_LOOPS) MAX_LOOPS="$value" ;;
        HARNESS_PID) HARNESS_PID="$value" ;;
      esac
    done < "$STATE_FILE"
  fi
}

state_write() {
  local tmp="$STATE_FILE.tmp"
  {
    echo "HARNESS_VERSION=${HARNESS_VERSION:-1}"
    echo "STATE=${STATE:-initial}"
    echo "SCENARIO=${SCENARIO:-}"
    echo "LOOP=${LOOP:-0}"
    echo "MAX_LOOPS=${MAX_LOOPS:-5}"
    echo "HARNESS_PID=${HARNESS_PID:-}"
  } > "$tmp"
  mv "$tmp" "$STATE_FILE"
}

state_reset() {
  rm -f "$STATE_FILE"
}

# --- PID lock ---

lock_acquire() {
  local my_pid=$$
  if [ -f "$PID_FILE" ]; then
    local existing_pid
    existing_pid=$(cat "$PID_FILE")
    if kill -0 "$existing_pid" 2>/dev/null; then
      echo "Error: Another harness instance is running (PID $existing_pid)." >&2
      exit 1
    fi
    msg "Removing stale PID file (PID $existing_pid no longer alive)."
  fi
  echo "$my_pid" > "$PID_FILE"
  HARNESS_PID=$my_pid
}

lock_release() {
  if [ -f "$PID_FILE" ] && [ "$(cat "$PID_FILE")" = "$$" ]; then
    rm -f "$PID_FILE"
  fi
}

lock_check() {
  if [ -f "$PID_FILE" ]; then
    local existing_pid
    existing_pid=$(cat "$PID_FILE")
    if [ "$existing_pid" != "$$" ] && kill -0 "$existing_pid" 2>/dev/null; then
      echo "Error: PID lock held by another process ($existing_pid)." >&2
      exit 1
    fi
  fi
}

# --- Stale .tmp cleanup ---

cleanup_stale_tmp() {
  local stale
  stale=$(find "$BASE_DIR" -maxdepth 1 -name '.harness-state.env.tmp' -mmin +60 2>/dev/null || true)
  if [ -n "$stale" ]; then
    rm -f "$stale"
  fi
}

# --- Precondition checks (for run.sh guarded transitions) ---

check_precondition() {
  local command="$1"
  local allowed_states="$2"
  state_read
  if [ "$STATE" = "initial" ]; then
    return 0
  fi
  local allowed=false
  for s in $allowed_states; do
    if [ "$STATE" = "$s" ]; then
      allowed=true
      break
    fi
  done
  if [ "$allowed" = false ]; then
    echo "Error: Cannot run '$command' in state '$STATE'. Allowed states: $allowed_states" >&2
    echo "Run 'eval/run.sh cleanup' or 'eval/harness.sh reset' to start fresh." >&2
    exit 1
  fi
}

# --- Feedback helpers ---

feedback_dir_for() {
  echo "$AGENT_TASKS_DIR/$1/feedback"
}

# --- Daemon lifecycle ---

check_daemon_health() {
  local pid_file="$1"
  local chit_home="$2"
  if [ ! -f "$pid_file" ]; then
    echo "Error: No PID file found at $pid_file" >&2
    return 1
  fi
  local pid
  pid=$(cat "$pid_file")
  if ! kill -0 "$pid" 2>/dev/null; then
    echo "Error: Daemon (PID $pid) is not running" >&2
    return 1
  fi
  sleep 1
  if ! env CHIT_HOME="$chit_home" "$CHIT_BIN" list &>/dev/null; then
    echo "Error: Daemon (PID $pid) is running but not responding to 'chit list'" >&2
    return 1
  fi
  msg "Daemon OK (PID $pid)"
  return 0
}

show_chit_version() {
  local version
  version=$("$CHIT_BIN" --version 2>/dev/null || echo "unknown")
  msg "chit version: $version"
}

stop_daemon() {
  if [ -f "$BASE_DIR/tmp/daemon.pid" ]; then
    local pid
    pid=$(cat "$BASE_DIR/tmp/daemon.pid")
    if kill -0 "$pid" 2>/dev/null; then
      msg "Stopping daemon (PID $pid)..."
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
    rm -f "$BASE_DIR/tmp/daemon.pid"
  else
    msg "No daemon PID file found."
  fi
}

# --- Scenario lifecycle ---

cleanup() {
  msg "Cleaning up temp directories..."
  rm -rf "$BASE_DIR/tmp" "$AGENT_TASKS_DIR"
  msg "Done."
}

clean_scenario() {
  local scenario="$1"
  msg "Cleaning previous $scenario run..."
  rm -rf "$BASE_DIR/tmp/$scenario" "$AGENT_TASKS_DIR/$scenario"
  if [ -f "$BASE_DIR/tmp/daemon.pid" ]; then
    local pid
    pid=$(cat "$BASE_DIR/tmp/daemon.pid")
    if kill -0 "$pid" 2>/dev/null; then
      msg "Stopping stale daemon (PID $pid)..."
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
    rm -f "$BASE_DIR/tmp/daemon.pid"
  fi
}

collect_feedback() {
  local scenario="$1"
  local feedback_dir
  feedback_dir=$(feedback_dir_for "$scenario")
  stop_daemon
  hdr "$scenario eval: COLLECTED"
  local aggregated="$AGENT_TASKS_DIR/$scenario/aggregated-feedback.md"
  > "$aggregated"
  local count=0
  if [ -d "$feedback_dir" ]; then
    for f in "$feedback_dir"/*.md; do
      if [ -f "$f" ]; then
        local agent_name
        agent_name=$(basename "$f" .md)
        echo "--- $agent_name ---"
        cat "$f"
        echo ""
        {
          echo "## Feedback from $agent_name"
          echo ""
          cat "$f"
          echo ""
        } >> "$aggregated"
        count=$((count + 1))
      fi
    done
  fi
  if [ "$count" -eq 0 ]; then
    msg "No feedback files found in $feedback_dir"
    echo "No feedback was collected." >> "$aggregated"
  else
    msg "---"
    msg "Saved $count feedback file(s) in $feedback_dir"
    msg "Aggregated feedback written to $aggregated"
  fi
  msg ""
  msg "Next step: ./eval/run.sh critique $scenario"
}

critique_generate() {
  local scenario="$1"
  local feedback_dir
  feedback_dir=$(feedback_dir_for "$scenario")
  local title="$2"
  local specifics="$3"

  hdr "CRITIC PROMPT — $scenario"

  local feedback_content=""
  if [ -d "$feedback_dir" ]; then
    for f in "$feedback_dir"/*.md; do
      if [ -f "$f" ]; then
        feedback_content="$feedback_content
$(cat "$f")
"
      fi
    done
  fi

  if [ -z "$feedback_content" ]; then
    msg "WARNING: No feedback files found in $feedback_dir"
    msg "The agents may not have written their feedback files yet."
    msg "You can still manually paste feedback below."
    echo ""
    feedback_content="__FEEDBACK__"
  fi

  cat << CRITPROMPT
Copy this into a Task tool call for the critic sub-agent:

task description="Critic — $scenario" subagent_type="general" prompt="
# Critic — $title

You are evaluating feedback from agents that tested the **chit** agent-to-agent messaging tool.

## Collected Feedback

$feedback_content

## Your Task

Read the feedback above carefully. Cross-reference between agents and assess each item:

1. **Cross-reference** — identify where different agents report the same issue in different words
2. **Assess materiality** — would fixing this make a real, noticeable difference to the product?
3. **Classify** each item as:
   - **P0** — must fix (crashes, data loss, hangs, broken core flow)
   - **P1** — should fix (confusing UX, missing feature that blocks workflow)
   - **P2** — nice to have (polish, convenience, minor ergonomics)
4. **Recommend only material items** — exclude noise, one-off preferences, and non-actionable feedback
$specifics

Return your analysis as JSON matching this schema:
\`\`\`json
{
  "p0": [{"description": "...", "rationale": "..."}],
  "p1": [{"description": "...", "rationale": "..."}],
  "p2": [{"description": "...", "rationale": "..."}],
  "summary": "overall assessment"
}
\`\`\`

Write the JSON to a code block in your response.
---
"
CRITPROMPT

  if [ "$feedback_content" != "__FEEDBACK__" ]; then
    msg ""
    msg "Feedback was auto-injected from $feedback_dir"
    msg "If agents didn't write files, manually replace __FEEDBACK__ above."
  fi

  local critic_prompt="$AGENT_TASKS_DIR/$scenario/critic-prompt.md"
  mkdir -p "$AGENT_TASKS_DIR/$scenario"
  cat > "$critic_prompt" << CRITFILE
# Critic — $title

You are evaluating feedback from agents that tested the **chit** agent-to-agent messaging tool.

## Collected Feedback

$feedback_content

## Your Task

Read the feedback above carefully. Cross-reference between agents and assess each item:

1. **Cross-reference** — identify where different agents report the same issue in different words
2. **Assess materiality** — would fixing this make a real, noticeable difference to the product?
3. **Classify** each item as:
   - **P0** — must fix (crashes, data loss, hangs, broken core flow)
   - **P1** — should fix (confusing UX, missing feature that blocks workflow)
   - **P2** — nice to have (polish, convenience, minor ergonomics)
4. **Recommend only material items** — exclude noise, one-off preferences, and non-actionable feedback
$specifics

Return your analysis as JSON matching this schema:
\`\`\`json
{
  "p0": [{"description": "...", "rationale": "..."}],
  "p1": [{"description": "...", "rationale": "..."}],
  "p2": [{"description": "...", "rationale": "..."}],
  "summary": "overall assessment"
}
\`\`\`

Write the JSON to a code block in your response.
CRITFILE
  msg "Critic prompt written to $critic_prompt"
}
