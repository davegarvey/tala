#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

# --- Scenario functions (available to both run.sh and harness.sh) ---

setup_cross_project() {
  clean_scenario "cross-project"
  # Clean agent prompt files and feedback (but preserve critic output)
  rm -f "$AGENT_TASKS_DIR/cross-project"/agent-*.md
  rm -rf "$AGENT_TASKS_DIR/cross-project/feedback"
  local tmp_dir="$BASE_DIR/tmp/cross-project"
  mkdir -p "$tmp_dir"/{project-alpha,project-beta}

  # Write project-alpha seed
  cat > "$tmp_dir/project-alpha/README.md" << 'SEED'
# CSV Processor

Parses CSV files and outputs JSON. Currently has a bug in `parse_row()`
that causes incorrect field mapping for quoted fields.

## File: process.py

```python
import csv
import json
import sys

def parse_row(row):
    fields = row.split(',')
    return {"fields": fields}

def main():
    data = sys.stdin.read()
    rows = data.strip().split('\n')
    reader = csv.reader(rows)
    for row in reader:
        result = parse_row(row)
        print(json.dumps(result))

if __name__ == "__main__":
    main()
```

Test input:
```
name,age,city
Alice,30,"New York, NY"
Bob,25,"Los Angeles, CA"
```

Expected: quoted cities should be single fields, not split on internal comma.
SEED

  # Write project-beta seed
  cat > "$tmp_dir/project-beta/README.md" << 'SEED'
# Data Schema Docs

Documents the CSV schema used across projects.

## CSV Format Rules

- All fields are separated by commas
- Fields containing commas, newlines, or double-quotes must be wrapped in double-quotes
- A double-quote character inside a quoted field is escaped with another double-quote
- Fields may have leading/trailing whitespace, which should be preserved unless quoted

## Valid Parsing Approach

Use Python's `csv.reader` or equivalent — it handles all quoting rules correctly.
The bug is that `parse_row` does `row.split(',')` instead of using the `csv` module's
reader properly. The fix is to remove `parse_row` entirely and use `csv.reader` for
the actual parsing.
SEED

  # Create the process.py file
  cat > "$tmp_dir/project-alpha/process.py" << 'PY'
import csv
import json
import sys

def parse_row(row):
    fields = row.split(',')
    return {"fields": fields}

def main():
    data = sys.stdin.read()
    rows = data.strip().split('\n')
    reader = csv.reader(rows)
    for row in reader:
        result = parse_row(row)
        print(json.dumps(result))

if __name__ == "__main__":
    main()
PY

  # Write task files for the coding agent
  mkdir -p "$AGENT_TASKS_DIR/cross-project"
  local feedback_dir
  feedback_dir=$(feedback_dir_for "cross-project")
  mkdir -p "$feedback_dir"

  cat > "$AGENT_TASKS_DIR/cross-project/agent-alpha.md" << TASK
# Agent Alpha — Cross-Project Eval

You are in project-alpha at: $tmp_dir/project-alpha

## Your Role
You're a developer maintaining project-alpha. Your code depends on a library
maintained by the agent in project-beta. You've noticed a CSV parsing bug
and need to coordinate with them to get it fixed.

## This Is an Eval!
Your real job is to evaluate the **tala tool itself**. tala is an agent-to-agent
messaging tool. Try it out, explore its features, and report what worked and
what didn't. Your feedback directly shapes the product.

The tala binary is at: $TALA_BIN

First, change to your project directory — this ensures tala uses the right active session:
\`\`\`
cd $tmp_dir/project-alpha
export TALA_HOME=$tmp_dir/.tala
\`\`\`

## Scenario
1. Read README.md and process.py to understand the CSV parsing bug
2. Use tala to collaborate with the expert in project-beta
3. Apply the fix and verify it works

**But don't just follow a script** — explore tala's commands and see what you
discover. Try things like starting sessions, sending with and without flags,
checking session status, listing sessions, renaming, closing, using recap,
sending files, JSON output, timeout options, etc. This is your chance to kick
the tires.

### tala commands to explore
\`\`\`
tala start <message>          Start a new session
tala send <message>           Send a message (uses active session by default)
tala wait                     Wait for new messages (sets active session)
tala recap                    Read the full conversation
tala list                     List all sessions
tala status                   Show session status
tala use <id>                 Set the active session
tala close <id>               Close a session
tala session rename <id> <name>  Give a session a name
tala follow                   Stream new messages live
tala observe                  Watch all sessions (multi-agent)
\`\`\`

Try as many as you can. You don't need to use them all, but the more you
try, the better the feedback.

### Feedback (write to file + return inline)
After your collaboration, **write your feedback to the file below** AND include
it in your final message. The file is what gets fed into the product review, so
be thorough. Write the file first, then return the same content inline.

Feedback file path: $feedback_dir/alpha.md

Answer honestly:
- What commands and features did you try?
- Which were intuitive? Which were confusing?
- What was the most frustrating moment?
- What surprised you (good or bad)?
- If you could change one thing, what would it be?
- Did using tala feel natural for agent-to-agent collaboration?

Start your file and inline response with:
## Feedback from Agent Alpha (project-alpha)
TASK

  cat > "$AGENT_TASKS_DIR/cross-project/agent-beta.md" << TASK
# Agent Beta — Cross-Project Eval

You are in project-beta at: $tmp_dir/project-beta

## Your Role
You're a domain expert on the CSV schema used across projects. The agent in
project-alpha maintains a library that depends on your project, and they've
found a bug they need your help with.

## This Is an Eval!
Your real job is to evaluate the **tala tool itself**. tala is an agent-to-agent
messaging tool. Try it out, explore its features, and report what worked and
what didn't. Your feedback directly shapes the product.

The tala binary is at: $TALA_BIN

First, change to your project directory — this ensures tala uses the right active session:
\`\`\`
cd $tmp_dir/project-beta
export TALA_HOME=$tmp_dir/.tala
\`\`\`

## Scenario
1. Read README.md to understand the CSV data format
2. Watch for a message from project-alpha via tala
3. Diagnose the bug and help them fix it

**But don't just follow a script** — explore tala's commands and see what you
discover. Try things like waiting for messages with options, checking session
status, listing active sessions, sending files, using recap to review the full
conversation, renaming sessions, JSON output, etc. This is your chance to
kick the tires.

### tala commands to explore
\`\`\`
tala wait                     Wait for new messages (sets active session)
tala send <message>           Send a message (uses active session by default)
tala recap                    Read the full conversation
tala list                     List all sessions
tala status                   Show session status
tala use <id>                 Set the active session
tala close <id>               Close a session
tala session rename <id> <name>  Give a session a name
tala follow                   Stream new messages live
tala start <message>          Start a new session
tala observe                  Watch all sessions (multi-agent)
\`\`\`

Try as many as you can. You don't need to use them all, but the more you
try, the better the feedback.

### Feedback (write to file + return inline)
After your collaboration, **write your feedback to the file below** AND include
it in your final message. The file is what gets fed into the product review, so
be thorough. Write the file first, then return the same content inline.

Feedback file path: $feedback_dir/beta.md

Answer honestly:
- What commands and features did you try?
- Which were intuitive? Which were confusing?
- What was the most frustrating moment?
- What surprised you (good or bad)?
- If you could change one thing, what would it be?
- Did using tala feel natural for agent-to-agent collaboration?

Start your file and inline response with:
## Feedback from Agent Beta (project-beta)
TASK

  # Start the daemon (nohup + disown so the bash tool doesn't kill it on timeout)
  TALA_HOME="$tmp_dir/.tala" nohup "$TALA_BIN" daemon > /dev/null 2>&1 &
  disown
  local daemon_pid=$!
  echo $daemon_pid > "$BASE_DIR/tmp/daemon.pid"
  msg "Starting daemon..."

  if ! check_daemon_health "$BASE_DIR/tmp/daemon.pid" "$tmp_dir/.tala"; then
    echo "Error: Daemon failed to start. Aborting."
    exit 1
  fi

  show_tala_version

  hdr "cross-project eval: READY"
  msg ""
  msg "Copy these into parallel Task tool calls:"
  echo ""
  while IFS= read -r line; do echo "$line"; done < "$AGENT_TASKS_DIR/cross-project/agent-alpha.md" | \
    awk '/^# Agent Alpha/{p=1} p{print}'
  echo '```'
  echo 'task description="Eval Agent Alpha" subagent_type="general" prompt="'
  cat "$AGENT_TASKS_DIR/cross-project/agent-alpha.md" | sed 's/"/\\"/g'
  echo '"'
  echo '```'
  echo ""
  echo "---"
  echo ""
  while IFS= read -r line; do echo "$line"; done < "$AGENT_TASKS_DIR/cross-project/agent-beta.md" | \
    awk '/^# Agent Beta/{p=1} p{print}'
  echo '```'
  echo 'task description="Eval Agent Beta" subagent_type="general" prompt="'
  cat "$AGENT_TASKS_DIR/cross-project/agent-beta.md" | sed 's/"/\\"/g'
  echo '"'
  echo '```'
  echo ""
  echo "TALA_HOME=$tmp_dir/.tala"
  echo "Daemon PID: $(cat $BASE_DIR/tmp/daemon.pid)"
  echo ""
  msg "After both finish:  ./eval/run.sh collect cross-project"
}

collect_cross_project() {
  collect_feedback "cross-project"
}

setup_observe() {
  clean_scenario "observe"
  local tmp_dir="$BASE_DIR/tmp/observe"
  mkdir -p "$tmp_dir"/{project-alpha,project-beta,project-gamma,monitor}

  for proj in alpha beta gamma; do
    cat > "$tmp_dir/project-$proj/README.md" << SEED
# Project $proj

A simple component. Create the required file and verify it works.
When done, send a tala status update.
SEED
  done

  mkdir -p "$AGENT_TASKS_DIR/observe"
  local feedback_dir
  feedback_dir=$(feedback_dir_for "observe")
  mkdir -p "$feedback_dir"

  cat > "$AGENT_TASKS_DIR/observe/agent-alpha.md" << TASK
# Agent Alpha — Observe Eval

You are in project-alpha at: $tmp_dir/project-alpha

## Your Task

First, change to your project directory — this ensures tala uses the right active session:
\`\`\`
cd $tmp_dir/project-alpha
export TALA_HOME=$tmp_dir/.tala
\`\`\`

Create \`src/server.py\` with a health-check endpoint that returns:
\`\`\`python
{"status": "ok", "version": "1.0.0"}
\`\`\`

Use tala to send status updates as you work (start, done, etc).
All tala commands must be run from $tmp_dir/project-alpha.

### Feedback (write to file + return inline)
After your task, **write your feedback to the file below** AND include it in
your final message. Write the file first, then return the same content inline.

Feedback file path: $feedback_dir/alpha.md

Answer:
- How easy was it to get started with tala?
- How intuitive were the commands?
- Was anything confusing or surprising?
- What would you improve?

Start your file and inline response with:
## Feedback from Agent Alpha (project-alpha)
TASK

  cat > "$AGENT_TASKS_DIR/observe/agent-beta.md" << TASK
# Agent Beta — Observe Eval

You are in project-beta at: $tmp_dir/project-beta

## Your Task

First, change to your project directory — this ensures tala uses the right active session:
\`\`\`
cd $tmp_dir/project-beta
export TALA_HOME=$tmp_dir/.tala
\`\`\`

Create \`src/watch.py\` that watches a file path and prints changes.
Use tala to send status updates.
All tala commands must be run from $tmp_dir/project-beta.

### Feedback (write to file + return inline)
After your task, **write your feedback to the file below** AND include it in
your final message. Write the file first, then return the same content inline.

Feedback file path: $feedback_dir/beta.md

Answer:
- How easy was it to get started with tala?
- How intuitive were the commands?
- Was anything confusing or surprising?
- What would you improve?

Start your file and inline response with:
## Feedback from Agent Beta (project-beta)
TASK

  cat > "$AGENT_TASKS_DIR/observe/agent-gamma.md" << TASK
# Agent Gamma — Observe Eval

You are in project-gamma at: $tmp_dir/project-gamma

## Your Task

First, change to your project directory — this ensures tala uses the right active session:
\`\`\`
cd $tmp_dir/project-gamma
export TALA_HOME=$tmp_dir/.tala
\`\`\`

Write documentation (README.md) for "ChitChat" — a fictional messaging API.
Include title, description, and usage section.
Use tala to send status updates.
All tala commands must be run from $tmp_dir/project-gamma.

### Feedback (write to file + return inline)
After your task, **write your feedback to the file below** AND include it in
your final message. Write the file first, then return the same content inline.

Feedback file path: $feedback_dir/gamma.md

Answer:
- How easy was it to get started with tala?
- How intuitive were the commands?
- Was anything confusing or surprising?
- What would you improve?

Start your file and inline response with:
## Feedback from Agent Gamma (project-gamma)
TASK

  cat > "$AGENT_TASKS_DIR/observe/monitor.md" << TASK
# Monitor — Observe Eval

You are the monitor, watching all agent activity.

## Your Task

First, change to the monitor directory — this ensures tala uses the right active session:
\`\`\`
cd $tmp_dir/monitor
export TALA_HOME=$tmp_dir/.tala
\`\`\`

Run \`tala observe\` and watch the three agents work.
Note what you can see — do you have enough context to understand each project?

### Feedback (write to file + return inline)
After observing, **write your feedback to the file below** AND include it in
your final message. Write the file first, then return the same content inline.

Feedback file path: $feedback_dir/monitor.md

Answer:
- Did \`tala observe\` give you an accurate picture of what was happening?
- Could you distinguish between the different sessions/agents?
- What would make observe more useful?
- How did you discover the observe command? Was it intuitive?
- How easy was it to get started with tala?
- How intuitive were the commands?

Start your file and inline response with:
## Feedback from Monitor
TASK

  # Start daemon (nohup + disown so the bash tool doesn't kill it on timeout)
  TALA_HOME="$tmp_dir/.tala" nohup "$TALA_BIN" daemon > /dev/null 2>&1 &
  disown
  local daemon_pid=$!
  echo $daemon_pid > "$BASE_DIR/tmp/daemon.pid"
  msg "Starting daemon..."

  if ! check_daemon_health "$BASE_DIR/tmp/daemon.pid" "$tmp_dir/.tala"; then
    echo "Error: Daemon failed to start. Aborting."
    exit 1
  fi

  show_tala_version

  hdr "observe eval: READY"
  msg ""
  msg "Launch all in parallel: Alpha, Beta, Gamma, and Monitor"
  echo ""
  echo "### Agent Alpha prompt"
  echo '```'
  cat "$AGENT_TASKS_DIR/observe/agent-alpha.md"
  echo '```'
  echo ""
  echo "### Agent Beta prompt"
  echo '```'
  cat "$AGENT_TASKS_DIR/observe/agent-beta.md"
  echo '```'
  echo ""
  echo "### Agent Gamma prompt"
  echo '```'
  cat "$AGENT_TASKS_DIR/observe/agent-gamma.md"
  echo '```'
  echo ""
  echo "### Monitor prompt (run last)"
  echo '```'
  cat "$AGENT_TASKS_DIR/observe/monitor.md"
  echo '```'
  echo ""
  echo "TALA_HOME=$tmp_dir/.tala"
  echo "Daemon PID: $(cat $BASE_DIR/tmp/daemon.pid)"
  echo ""
  msg "After all finish:  ./eval/run.sh collect observe"
}

collect_observe() {
  collect_feedback "observe"
}

critique_cross_project() {
  critique_generate "cross-project" "Cross-Project Eval" ""
}

critique_observe() {
  critique_generate "observe" "Observe Eval" "- The feedback is specifically about the \`tala observe\` feature — pay special attention to multi-agent monitoring concerns"
}

# State-aware dispatch with precondition checks
# When .harness-state.env exists, enforce transition order.
# When absent, operate in backward-compatible mode (no guards).
# Source guard: only run dispatch when executed directly, not when sourced.
if [ "${BASH_SOURCE[0]}" = "$0" ]; then
cleanup_stale_tmp
lock_acquire
trap lock_release EXIT
case "${1:-help}" in
  setup)
    if [ -z "${2:-}" ]; then
      echo "Usage: $0 setup <scenario>"
      echo "Scenarios: cross-project, observe"
      exit 1
    fi
    mkdir -p "$BASE_DIR/tmp"
    if [ -f "$STATE_FILE" ]; then
      check_precondition "setup" "initial"
    fi
    "setup_${2//-/_}"
    STATE=launching
    SCENARIO="$2"
    state_write
    ;;
  collect)
    if [ -z "${2:-}" ]; then
      echo "Usage: $0 collect <scenario>"
      exit 1
    fi
    if [ -f "$STATE_FILE" ]; then
      check_precondition "collect" "launching"
    fi
    "collect_${2//-/_}"
    STATE=collecting
    state_write
    ;;
  critique)
    if [ -z "${2:-}" ]; then
      echo "Usage: $0 critique <scenario>"
      exit 1
    fi
    if [ -f "$STATE_FILE" ]; then
      check_precondition "critique" "collecting"
    fi
    "critique_${2//-/_}"
    STATE=critiquing
    state_write
    ;;
  cleanup)
    stop_daemon
    cleanup
    state_reset
    ;;
  *)
    echo "Usage: $0 {setup|collect|critique|cleanup} [scenario]"
    echo ""
    echo "Commands:"
    echo "  setup <scenario>    Prepare environment and launch daemon"
    echo "  collect <scenario>  Gather feedback and stop daemon"
    echo "  critique <scenario> Run critic sub-agent on collected feedback"
    echo "  cleanup             Remove all temp files"
    echo ""
    echo "Scenarios: cross-project observe"
    exit 1
    ;;
esac
fi
