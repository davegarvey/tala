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

## Context
You have a CSV parser bug to fix. A data-schema expert (project-beta) can help.
Use chit to communicate with them. The chit binary is at: $CHIT_BIN

Before starting, make sure chit is on your PATH or use the full path above.

## Your Task
1. Read README.md and process.py to understand the bug
2. Use chit to start a session and send a message to the other agent describing the bug
3. Wait for their reply, apply the fix, verify it works
4. Write feedback to $RESULTS_DIR/cross-project-feedback.md (append, don't overwrite)

### Suggested chit workflow
\`\`\`
chit start "Help: CSV parser bug with quoted fields"
chit send --session <id> "row.split(',') breaks on 'New York, NY'"
chit wait --session <id>
\`\`\`

### Feedback questions to answer
- How easy was it to start using chit?
- How intuitive were send, wait, recap?
- Was there any confusion about the API (flags, defaults, session management)?
- What was the most frustrating part?
- What would you change or improve?
- Did the tool help or hinder collaboration?

Start your feedback with:
## Feedback from Agent Alpha (project-alpha)
TASK

  cat > "$AGENT_TASKS_DIR/cross-project/agent-beta.md" << TASK
# Agent Beta — Cross-Project Eval

You are in project-beta at: $tmp_dir/project-beta

## Context
You are a data-schema expert. An agent in project-alpha will contact you via chit
about a CSV parsing bug. Help them fix it.

The chit binary is at: $CHIT_BIN

## Your Task
1. Read README.md to understand the CSV data format
2. Wait for a message from project-alpha via chit (use \`chit wait\` or check for messages)
3. Diagnose the bug and tell them the exact fix
4. Write feedback to $RESULTS_DIR/cross-project-feedback.md (append, don't overwrite)

### Suggested chit workflow
\`\`\`
# Wait for someone to contact you (they'll start a session first)
chit list --json   # see what sessions exist
chit recap --session <id>   # read the messages
chit send --session <id> "The fix is to remove parse_row and use csv.reader directly"
\`\`\`

### Feedback questions to answer
- How easy was it to receive and reply to messages?
- How intuitive were the commands?
- Was anything confusing or surprising?
- What would you improve?
- Did the tool help or hinder collaboration?

Start your feedback with:
## Feedback from Agent Beta (project-beta)
TASK

  # Start the daemon
  CHIT_HOME="$tmp_dir/.chit" $CHIT_BIN daemon 2>/dev/null &
  echo $! > "$BASE_DIR/tmp/daemon.pid"
  sleep 1

  echo "==========================================="
  echo "  cross-project eval: READY"
  echo "==========================================="
  echo ""
  echo "Launch these sub-agents via the Task tool:"
  echo ""
  echo "  1. Agent Alpha  →  $AGENT_TASKS_DIR/cross-project/agent-alpha.md"
  echo "  2. Agent Beta   →  $AGENT_TASKS_DIR/cross-project/agent-beta.md"
  echo ""
  echo "Chit daemon running with CHIT_HOME=$tmp_dir/.chit"
  echo "PID: $(cat $BASE_DIR/tmp/daemon.pid)"
  echo ""
  echo "Once both agents finish, run:"
  echo "  ./eval/run.sh collect cross-project"
  echo "==========================================="
}

collect_cross_project() {
  local feedback_file="$RESULTS_DIR/cross-project-feedback.md"
  if [ ! -f "$feedback_file" ]; then
    echo "No feedback found at $feedback_file"
    exit 1
  fi
  echo "==========================================="
  echo "  cross-project eval: RESULTS"
  echo "==========================================="
  cat "$feedback_file"
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

  # Start daemon
  CHIT_HOME="$tmp_dir/.chit" $CHIT_BIN daemon 2>/dev/null &
  echo $! > "$BASE_DIR/tmp/daemon.pid"
  sleep 1

  echo "==========================================="
  echo "  observe eval: READY"
  echo "==========================================="
  echo ""
  echo "Launch these sub-agents via the Task tool:"
  echo ""
  echo "  1. Agent Alpha  →  $AGENT_TASKS_DIR/observe/agent-alpha.md"
  echo "  2. Agent Beta   →  $AGENT_TASKS_DIR/observe/agent-beta.md"
  echo "  3. Agent Gamma  →  $AGENT_TASKS_DIR/observe/agent-gamma.md"
  echo "  4. Monitor      →  $AGENT_TASKS_DIR/observe/monitor.md"
  echo ""
  echo "Launch in order: Alpha, Beta, Gamma, then Monitor (so monitor has activity to see)"
  echo "Chit daemon running with --home $tmp_dir/.chit"
  echo "PID: $(cat $BASE_DIR/tmp/daemon.pid)"
  echo ""
  echo "Once all finish, run:"
  echo "  ./eval/run.sh collect observe"
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
