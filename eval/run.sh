#!/usr/bin/env bash
set -euo pipefail

CHIT_BIN="${CHIT_BIN:-$(dirname "$0")/../target/release/chit}"
BASE_DIR="$(cd "$(dirname "$0")" && pwd)"
SCENARIOS_DIR="$BASE_DIR/scenarios"
AGENT_TASKS_DIR="$BASE_DIR/agent-tasks"
RESULTS_DIR="$BASE_DIR/results"

if [ ! -f "$CHIT_BIN" ]; then
  CHIT_BIN="$(dirname "$0")/../target/debug/chit"
fi
if [ ! -f "$CHIT_BIN" ]; then
  echo "Error: chit binary not found. Build with: cargo build --release"
  exit 1
fi

cleanup() {
  echo "Cleaning up temp directories..."
  rm -rf "$BASE_DIR/tmp" "$AGENT_TASKS_DIR" "$RESULTS_DIR"
  echo "Done."
}

setup_cross_project() {
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

  # Create results file
  mkdir -p "$RESULTS_DIR"
  > "$RESULTS_DIR/cross-project-feedback.md"

  # Write task files for the coding agent
  mkdir -p "$AGENT_TASKS_DIR/cross-project"

  cat > "$AGENT_TASKS_DIR/cross-project/agent-alpha.md" << TASK
# Agent Alpha — Cross-Project Eval

You are in project-alpha at: $tmp_dir/project-alpha

## Your Role
You're a developer maintaining project-alpha. Your code depends on a library
maintained by the agent in project-beta. You've noticed a CSV parsing bug
and need to coordinate with them to get it fixed.

## This Is an Eval!
Your real job is to evaluate the **chit tool itself**. chit is an agent-to-agent
messaging tool. Try it out, explore its features, and report what worked and
what didn't. Your feedback directly shapes the product.

The chit binary is at: $CHIT_BIN

Before starting, export CHIT_HOME:
\`\`\`
export CHIT_HOME=$tmp_dir/.chit
\`\`\`

## Scenario
1. Read README.md and process.py to understand the CSV parsing bug
2. Use chit to collaborate with the expert in project-beta
3. Apply the fix and verify it works

**But don't just follow a script** — explore chit's commands and see what you
discover. Try things like starting sessions, sending with and without flags,
checking session status, listing sessions, renaming, closing, using recap,
sending files, JSON output, timeout options, etc. This is your chance to kick
the tires.

### chit commands to explore
\`\`\`
chit start <message>          Start a new session
chit send <message>           Send a message (uses active session by default)
chit wait                     Wait for new messages (sets active session)
chit recap                    Read the full conversation
chit list                     List all sessions
chit status                   Show session status
chit use <id>                 Set the active session
chit close <id>               Close a session
chit session rename <id> <name>  Give a session a name
chit follow                   Stream new messages live
chit observe                  Watch all sessions (multi-agent)
\`\`\`

Try as many as you can. You don't need to use them all, but the more you
try, the better the feedback.

### Feedback (return inline)
After your collaboration, return your feedback as part of your final message
(not written to a file). Answer honestly:

- What commands and features did you try?
- Which were intuitive? Which were confusing?
- What was the most frustrating moment?
- What surprised you (good or bad)?
- If you could change one thing, what would it be?
- Did using chit feel natural for agent-to-agent collaboration?

Start your feedback with:
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
Your real job is to evaluate the **chit tool itself**. chit is an agent-to-agent
messaging tool. Try it out, explore its features, and report what worked and
what didn't. Your feedback directly shapes the product.

The chit binary is at: $CHIT_BIN

Before starting, export CHIT_HOME:
\`\`\`
export CHIT_HOME=$tmp_dir/.chit
\`\`\`

## Scenario
1. Read README.md to understand the CSV data format
2. Watch for a message from project-alpha via chit
3. Diagnose the bug and help them fix it

**But don't just follow a script** — explore chit's commands and see what you
discover. Try things like waiting for messages with options, checking session
status, listing active sessions, sending files, using recap to review the full
conversation, renaming sessions, JSON output, etc. This is your chance to
kick the tires.

### chit commands to explore
\`\`\`
chit wait                     Wait for new messages (sets active session)
chit send <message>           Send a message (uses active session by default)
chit recap                    Read the full conversation
chit list                     List all sessions
chit status                   Show session status
chit use <id>                 Set the active session
chit close <id>               Close a session
chit session rename <id> <name>  Give a session a name
chit follow                   Stream new messages live
chit start <message>          Start a new session
chit observe                  Watch all sessions (multi-agent)
\`\`\`

Try as many as you can. You don't need to use them all, but the more you
try, the better the feedback.

### Feedback (return inline)
After your collaboration, return your feedback as part of your final message
(not written to a file). Answer honestly:

- What commands and features did you try?
- Which were intuitive? Which were confusing?
- What was the most frustrating moment?
- What surprised you (good or bad)?
- If you could change one thing, what would it be?
- Did using chit feel natural for agent-to-agent collaboration?

Start your feedback with:
## Feedback from Agent Beta (project-beta)
TASK

  # Start the daemon (nohup + disown so the bash tool doesn't kill it on timeout)
  CHIT_HOME="$tmp_dir/.chit" nohup "$CHIT_BIN" daemon > /dev/null 2>&1 &
  disown
  echo $! > "$BASE_DIR/tmp/daemon.pid"
  sleep 1

  echo "==========================================="
  echo "  cross-project eval: READY"
  echo "==========================================="
  echo ""
  echo "Copy these into parallel Task tool calls:"
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
  echo "CHIT_HOME=$tmp_dir/.chit"
  echo "Daemon PID: $(cat $BASE_DIR/tmp/daemon.pid)"
  echo ""
  echo "After both finish:  ./eval/run.sh collect cross-project"
  echo "==========================================="
}

collect_cross_project() {
  echo "==========================================="
  echo "  cross-project eval: COMPLETE"
  echo "==========================================="
  echo "Feedback was returned inline by the Task agents."
  echo "Check the Task results above for agent feedback."
  echo "==========================================="
  stop_daemon
}

setup_observe() {
  local tmp_dir="$BASE_DIR/tmp/observe"
  mkdir -p "$tmp_dir"/{project-alpha,project-beta,project-gamma,monitor}

  for proj in alpha beta gamma; do
    cat > "$tmp_dir/project-$proj/README.md" << SEED
# Project $proj

A simple component. Create the required file and verify it works.
When done, send a chit status update.
SEED
  done

  mkdir -p "$RESULTS_DIR"
  > "$RESULTS_DIR/observe-feedback.md"

  mkdir -p "$AGENT_TASKS_DIR/observe"

  cat > "$AGENT_TASKS_DIR/observe/agent-alpha.md" << TASK
# Agent Alpha — Observe Eval

You are in project-alpha at: $tmp_dir/project-alpha

## Your Task
Create \`src/server.py\` with a health-check endpoint that returns:
\`\`\`python
{"status": "ok", "version": "1.0.0"}
\`\`\`

Use chit to send status updates as you work (start, done, etc).
Write feedback to $RESULTS_DIR/observe-feedback.md (append).

Feedback questions:
- How easy was it to get started with chit?
- How intuitive were the commands?
- Was anything confusing or surprising?
- What would you improve?

Start with:
## Feedback from Agent Alpha (project-alpha)
TASK

  cat > "$AGENT_TASKS_DIR/observe/agent-beta.md" << TASK
# Agent Beta — Observe Eval

You are in project-beta at: $tmp_dir/project-beta

## Your Task
Create \`src/watch.py\` that watches a file path and prints changes.
Use chit to send status updates.
Write feedback to $RESULTS_DIR/observe-feedback.md (append).

Same feedback questions as agent-alpha.

Start with:
## Feedback from Agent Beta (project-beta)
TASK

  cat > "$AGENT_TASKS_DIR/observe/agent-gamma.md" << TASK
# Agent Gamma — Observe Eval

You are in project-gamma at: $tmp_dir/project-gamma

## Your Task
Write documentation (README.md) for "ChitChat" — a fictional messaging API.
Include title, description, and usage section.
Use chit to send status updates.
Write feedback to $RESULTS_DIR/observe-feedback.md (append).

Same feedback questions as agent-alpha.

Start with:
## Feedback from Agent Gamma (project-gamma)
TASK

  cat > "$AGENT_TASKS_DIR/observe/monitor.md" << TASK
# Monitor — Observe Eval

You are the monitor, watching all agent activity.

## Your Task
Run \`chit observe\` from $tmp_dir/monitor and watch the three agents work.
Note what you can see — do you have enough context to understand each project?
Write feedback to $RESULTS_DIR/observe-feedback.md (append).

### Monitor-specific feedback
- Did \`chit observe\` give you an accurate picture of what was happening?
- Could you distinguish between the different sessions/agents?
- What would make observe more useful?
- How did you discover the observe command? Was it intuitive?

Start with:
## Feedback from Monitor
TASK

  # Start daemon (nohup + disown so the bash tool doesn't kill it on timeout)
  CHIT_HOME="$tmp_dir/.chit" nohup "$CHIT_BIN" daemon > /dev/null 2>&1 &
  disown
  echo $! > "$BASE_DIR/tmp/daemon.pid"
  sleep 1

  echo "==========================================="
  echo "  observe eval: READY"
  echo "==========================================="
  echo ""
  echo "Launch all in parallel: Alpha, Beta, Gamma, and Monitor"
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
  echo "CHIT_HOME=$tmp_dir/.chit"
  echo "Daemon PID: $(cat $BASE_DIR/tmp/daemon.pid)"
  echo ""
  echo "After all finish:  ./eval/run.sh collect observe"
  echo "==========================================="
}

collect_observe() {
  local feedback_file="$RESULTS_DIR/observe-feedback.md"
  if [ ! -f "$feedback_file" ]; then
    echo "No feedback found at $feedback_file"
    exit 1
  fi
  echo "==========================================="
  echo "  observe eval: RESULTS"
  echo "==========================================="
  cat "$feedback_file"
  echo "==========================================="
  stop_daemon
}

stop_daemon() {
  if [ -f "$BASE_DIR/tmp/daemon.pid" ]; then
    local pid
    pid=$(cat "$BASE_DIR/tmp/daemon.pid")
    if kill -0 "$pid" 2>/dev/null; then
      echo "Stopping daemon (PID $pid)..."
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
    rm -f "$BASE_DIR/tmp/daemon.pid"
  else
    echo "No daemon PID file found."
  fi
}

case "${1:-help}" in
  setup)
    if [ -z "${2:-}" ]; then
      echo "Usage: $0 setup <scenario>"
      echo "Scenarios: cross-project, observe"
      exit 1
    fi
    mkdir -p "$BASE_DIR/tmp"
    "setup_${2//-/_}"
    ;;
  collect)
    if [ -z "${2:-}" ]; then
      echo "Usage: $0 collect <scenario>"
      exit 1
    fi
    "collect_${2//-/_}"
    ;;
  cleanup)
    stop_daemon
    cleanup
    ;;
  *)
    echo "Usage: $0 {setup|collect|cleanup} [scenario]"
    echo ""
    echo "Commands:"
    echo "  setup <scenario>    Prepare environment and launch daemon"
    echo "  collect <scenario>  Gather feedback and stop daemon"
    echo "  cleanup             Remove all temp files"
    echo ""
    echo "Scenarios: cross-project observe"
    exit 1
    ;;
esac
