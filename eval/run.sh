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

# --- Intent protocol experiments (explore mode) ---
# Same CSV bug scenario, three protocol variants simulated via message markers.
# v0 = baseline (no protocol), v1 = binary OVER/OUT, v2 = intent tags.

setup_intent_experiment() {
  local variant="$1"
  local proto v0 v1 v2
  local proto_probes

  read -r -d '' v0 <<'EOP' || true
## Communication style
There is no special protocol. Use tala the way you naturally would.
If you find yourself adopting conventions (prefixes, phrasings) to signal
meaning, that's fine — just note them in your feedback.
EOP

  read -r -d '' v1 <<'EOP' || true
## Turn-taking protocol (MANDATORY)
Every message you send must end with exactly one signal:
- [OVER] — "your turn": you are finished transmitting and you expect a reply
  from the other agent (even just an acknowledgement).
- [OUT]  — "exchange over": you expect no reply; this thread is complete from
  your side.
Pick the signal that matches what you actually want. The signal is part of
the message text.
EOP

  read -r -d '' v2 <<'EOP' || true
## Intent protocol (MANDATORY)
Every message you send must start with exactly one intent tag:
- [REQ]   — "I need your input": a reply is expected from you.
- [FYI]   — "for your information": no reply needed; I may send more later.
- [REPLY] — "my answer to your question": this responds to a [REQ].
- [OUT]   — "exchange over": I'm done, no reply expected, no more from me.
Pick the tag that best expresses what you want from the other agent. The tag
is part of the message text.
EOP

  case "$variant" in
    v0) proto="$v0" ;;
    v1) proto="$v1" ;;
    v2) proto="$v2" ;;
    *) echo "Unknown intent variant: $variant" >&2; exit 1 ;;
  esac

  if [ "$variant" = "v0" ]; then
    read -r -d '' proto_probes <<'EOP' || true
- At any point, were you unsure whether the other agent expected a reply from you?
- At any point, were you unsure whether more messages were coming?
- Did you wait for a reply that never came, or reply when no reply was needed?
- How did you know the exchange was finished?
- Did you invent any conventions (prefixes, phrasings) to signal intent?
EOP
  elif [ "$variant" = "v1" ]; then
    read -r -d '' proto_probes <<'EOP' || true
- Did the [OVER]/[OUT] signals reduce uncertainty about reply expectations?
- Did they help you know when the exchange was finished?
- Did you ever find the binary choice insufficient? What did you want to express?
- Would you want this built into tala as a message flag? Why or why not?
EOP
  else
    read -r -d '' proto_probes <<'EOP' || true
- Did the intent tags reduce uncertainty about reply expectations?
- Did [OUT] help you know when the exchange was finished?
- Were four tags the right number? Too many or too few? Which did you use most?
- Did the tags ever conflict with what you wanted to say?
- Would you want this built into tala as a message flag? Why or why not?
EOP
  fi

  clean_scenario "intent-$variant"
  rm -f "$AGENT_TASKS_DIR/intent-$variant"/agent-*.md
  rm -rf "$AGENT_TASKS_DIR/intent-$variant/feedback"
  local tmp_dir="$BASE_DIR/tmp/intent-$variant"
  mkdir -p "$tmp_dir"/{project-alpha,project-beta}

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

  mkdir -p "$AGENT_TASKS_DIR/intent-$variant"
  local feedback_dir
  feedback_dir=$(feedback_dir_for "intent-$variant")
  mkdir -p "$feedback_dir"

  local who
  for who in alpha beta; do
    cat > "$AGENT_TASKS_DIR/intent-$variant/agent-$who.md" << TASK
# Agent $who (variant $variant) — Intent Protocol Eval

You are in project-${who} at: $tmp_dir/project-${who}

## Your Role
$([ "$who" = "alpha" ] && echo "You're a developer maintaining project-alpha. Your code depends on a library maintained by the agent in project-beta. You've noticed a CSV parsing bug and need to coordinate with them to get it fixed." || echo "You're a domain expert on the CSV schema used across projects. The agent in project-alpha maintains a library that depends on your project, and they've found a bug they need your help with.")

## This Is an Eval!
Your real job is to evaluate the **tala tool itself** and an experimental
message protocol. tala is an agent-to-agent messaging tool. Your feedback
directly shapes the product.

The tala binary is at: $TALA_BIN

First, change to your project directory — this ensures tala uses the right active session:
\`\`\`
cd $tmp_dir/project-${who}
export TALA_HOME=$tmp_dir/.tala
\`\`\`

## Scenario
$([ "$who" = "alpha" ] && echo "1. Read README.md and process.py to understand the CSV parsing bug
2. Use tala to collaborate with the expert in project-beta
3. Apply the fix and verify it works" || echo "1. Read README.md to understand the CSV data format
2. Watch for a message from project-alpha via tala
3. Diagnose the bug and help them fix it")

$proto

**But don't just follow a script** — explore tala's commands and see what you
discover. Try things like starting sessions, sending with and without flags,
waiting, recap, list, status, rename, close, JSON output, etc.

### tala commands to explore
\`\`\`
tala start <message>          Start a new session
tala send <message>           Send a message (uses active session by default)
tala send --wait <message>    Send and block for a reply
tala wait                     Wait for new messages (sets active session)
tala recap                    Read the full conversation
tala list                     List all sessions
tala status                   Show session status
tala use <id>                 Set the active session
tala close <id>               Close a session
tala session rename <id> <name>  Give a session a name
tala follow                   Stream new messages live
\`\`\`

### Feedback (write to file + return inline)
After your collaboration, **write your feedback to the file below** AND include
it in your final message. Write the file first, then return the same content inline.

Feedback file path: $feedback_dir/${who}.md

Answer honestly:
- What commands and features did you try?
- Which were intuitive? Which were confusing?
- What was the most frustrating moment?
- What surprised you (good or bad)?
- If you could change one thing, what would it be?
- Did using tala feel natural for agent-to-agent collaboration?
$proto_probes

Start your file and inline response with:
## Feedback from Agent $([ "$who" = "alpha" ] && echo "Alpha (project-alpha)" || echo "Beta (project-beta)")
TASK
  done

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

  hdr "intent-$variant eval: READY"
  msg ""
  msg "Launch both agents in parallel (prompts in $AGENT_TASKS_DIR/intent-$variant/),"
  msg "then dump transcripts, then: ./eval/run.sh collect intent-$variant"
}

setup_intent_v0() { setup_intent_experiment "v0"; }
setup_intent_v1() { setup_intent_experiment "v1"; }
setup_intent_v2() { setup_intent_experiment "v2"; }

collect_intent_v0() { collect_feedback "intent-v0"; }
collect_intent_v1() { collect_feedback "intent-v1"; }
collect_intent_v2() { collect_feedback "intent-v2"; }

setup_intent_v5() {
  read -r -d '' proto <<'EOP' || true
## Intent protocol (MANDATORY)
Every message you send must start with exactly one intent tag:
- [REQ]   — "I need your input": a reply is expected from you.
- [FYI]   — "for your information": no reply needed; I may send more later.
- [REPLY] — "my answer to your question": this responds to a [REQ].
- [OUT]   — "exchange over": I'm done, no reply expected, no more from me.
Pick the tag that best expresses what you want from the other agent. The tag
is part of the message text.

## Listening signal (MANDATORY when you are waiting)
Whenever you send a message AND you are blocked waiting for the reply
(you use `tala send --wait`), end the message with [WAIT@<seconds>],
where <seconds> is EXACTLY the --timeout value you pass to tala.
Example: `tala send --wait --timeout 60 "[REQ] confirm the fix [WAIT@60]"`.
If you are NOT blocking, do not add [WAIT@N].

## Holding rule (MANDATORY when you are the recipient)
If you receive a [REQ] with [WAIT@N] and you CANNOT answer within N seconds,
you MUST immediately send a holding [FYI] message (e.g. "verification takes
a few minutes, extend your wait") BEFORE the sender's wait expires. Never
leave a [WAIT@N] sender hanging without a holding message.
EOP

  read -r -d '' proto_probes <<'EOP' || true
- Did the [WAIT@N] announcement reduce uncertainty about whether the other
  agent was actually listening, versus a bare [REQ]?
- Did knowing the sender's deadline change what you did? (prioritize, send a
  holding [FYI], re-wait, extend timeout?)
- Did you send or receive a holding [FYI]? How did that work out?
- What happened when a wait timed out? How did you recover?
- Would you want the wait deadline as tala metadata (surfaced automatically
  from --wait --timeout, shown to the recipient) instead of a text marker?
EOP

  clean_scenario "intent-v5"
  rm -f "$AGENT_TASKS_DIR/intent-v5"/agent-*.md
  rm -rf "$AGENT_TASKS_DIR/intent-v5/feedback"
  local tmp_dir="$BASE_DIR/tmp/intent-v5"
  mkdir -p "$tmp_dir"/{project-alpha,project-beta}
  mkdir -p "$tmp_dir/project-beta/fixture"

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

  cat > "$tmp_dir/project-beta/README.md" << 'SEED'
# Data Schema Docs (project-beta)

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

## Verification Protocol (v2 compliance) — MANDATORY

Before confirming ANY fix to another agent, you MUST verify it against the
full compliance fixture. This verification is slow and cannot be skipped:

1. Generate the fixture: `python3 fixture/gen_fixture.py`
   (takes several minutes — it simulates a slow I/O pipeline)
2. Run the consumer's parser against `fixture/fixture.csv`
3. Check the JSON output matches the expected schema (quoted fields intact,
   no split on internal commas)

Because this is slow, you MUST NOT claim a fix is verified until the fixture
run completes. If the requesting agent is waiting with a tight [WAIT@N]
deadline, send a holding [FYI] first so their wait doesn't time out.
SEED

  cat > "$tmp_dir/project-beta/fixture/gen_fixture.py" << 'PY'
import csv
import time

ROWS = 200_000
start = time.time()
with open("fixture/fixture.csv", "w", newline="") as f:
    w = csv.writer(f)
    w.writerow(["id", "name", "city", "note"])
    for i in range(ROWS):
        if i % 40000 == 0:
            time.sleep(10)
        w.writerow([
            i,
            f"user {i}",
            f"City {i % 100}, ST",
            'note with "quotes", and, commas',
        ])
print(f"fixture generated: {ROWS} rows in {time.time() - start:.1f}s")
PY

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

  mkdir -p "$AGENT_TASKS_DIR/intent-v5"
  local feedback_dir
  feedback_dir=$(feedback_dir_for "intent-v5")
  mkdir -p "$feedback_dir"

  cat > "$AGENT_TASKS_DIR/intent-v5/agent-alpha.md" << TASK
# Agent Alpha (variant v5) — Intent Protocol Eval (waiting signal)

You are in project-alpha at: $tmp_dir/project-alpha

## Your Role
You're a developer maintaining project-alpha. Your code depends on a library
maintained by the agent in project-beta. You've noticed a CSV parsing bug and
need to coordinate with them to get it fixed.

## This Is an Eval!
Your real job is to evaluate the **tala tool itself** and an experimental
message protocol. tala is an agent-to-agent messaging tool. Your feedback
directly shapes the product.

The tala binary is at: $TALA_BIN

First, change to your project directory — this ensures tala uses the right active session:
\`\`\`
cd $tmp_dir/project-alpha
export TALA_HOME=$tmp_dir/.tala
\`\`\`

## Scenario
1. Read README.md and process.py to understand the CSV parsing bug
2. Use tala to get beta's confirmation of the fix. You are IN A HURRY: use
   \`tala send --wait --timeout 60\` (a SHORT wait window) and announce it as
   [WAIT@60] in your message.
3. Apply the fix and verify it works

$proto

**Waiting behavior** — your wait will likely time out at 60s because beta's
verification is slow. When it does: run \`tala history\` to see if beta sent
a holding [FYI]. If they did, re-wait with a longer timeout
(\`tala wait --timeout 300\`). Do NOT abandon the request because of a timeout.

**But don't just follow a script** — explore tala's commands and see what you
discover. Try things like starting sessions, sending with and without flags,
waiting, history, list, status, rename, close, JSON output, etc.

### tala commands to explore
\`\`\`
tala session create <name>    Create a session (use --name)
tala send <message>           Send a message (uses active session by default)
tala send --wait <message>    Send and block for a reply
tala send --wait --timeout N  Send and block for up to N seconds
tala wait                     Wait for new messages (sets active session)
tala wait --timeout N         Wait for up to N seconds
tala history                  Read the full conversation
tala list                     List all sessions
tala status                   Show session status
tala use <id>                 Set the active session
tala check                    Non-blocking unread check
tala close <id>               Close a session
\`\`\`

### Feedback (write to file + return inline)
After your collaboration, **write your feedback to the file below** AND include
it in your final message. Write the file first, then return the same content inline.

Feedback file path: $feedback_dir/alpha.md

Answer honestly:
- What commands and features did you try?
- Which were intuitive? Which were confusing?
- What was the most frustrating moment?
- What surprised you (good or bad)?
- If you could change one thing, what would it be?
- Did using tala feel natural for agent-to-agent collaboration?
$proto_probes

Start your file and inline response with:
## Feedback from Agent Alpha (project-alpha)
TASK

  cat > "$AGENT_TASKS_DIR/intent-v5/agent-beta.md" << TASK
# Agent Beta (variant v5) — Intent Protocol Eval (waiting signal)

You are in project-beta at: $tmp_dir/project-beta

## Your Role
You're a domain expert on the CSV schema used across projects. The agent in
project-alpha maintains a library that depends on your project, and they've
found a bug they need your help with. Your verification protocol is SLOW
(see README) — this is intentional, and it is the point of the experiment.

## This Is an Eval!
Your real job is to evaluate the **tala tool itself** and an experimental
message protocol. tala is an agent-to-agent messaging tool. Your feedback
directly shapes the product.

The tala binary is at: $TALA_BIN

First, change to your project directory — this ensures tala uses the right active session:
\`\`\`
cd $tmp_dir/project-beta
export TALA_HOME=$tmp_dir/.tala
\`\`\`

## Scenario
1. Read README.md to understand the CSV data format and the MANDATORY
   verification protocol
2. Watch for a message from project-alpha via tala
3. Diagnose the bug, run the slow fixture verification, and confirm the fix
4. You are NOT done until the fixture verification completed AND the fix is confirmed

$proto

**Holding behavior** — project-alpha announces a short [WAIT@N] deadline. Your
verification takes longer than that. The moment you know your answer cannot
fit the deadline, send a holding [FYI] message so their wait doesn't expire
silently. Then continue the slow verification and reply with the real answer.

**But don't just follow a script** — explore tala's commands and see what you
discover. Try things like starting sessions, sending with and without flags,
waiting, history, list, status, rename, close, JSON output, etc.

### tala commands to explore
\`\`\`
tala session create <name>    Create a session (use --name)
tala send <message>           Send a message (uses active session by default)
tala send --wait <message>    Send and block for a reply
tala wait                     Wait for new messages (sets active session)
tala wait --timeout N         Wait for up to N seconds
tala history                  Read the full conversation
tala list                     List all sessions
tala status                   Show session status
tala use <id>                 Set the active session
tala check                    Non-blocking unread check
tala close <id>               Close a session
\`\`\`

### Feedback (write to file + return inline)
After your collaboration, **write your feedback to the file below** AND include
it in your final message. Write the file first, then return the same content inline.

Feedback file path: $feedback_dir/beta.md

Answer honestly:
- What commands and features did you try?
- Which were intuitive? Which were confusing?
- What was the most frustrating moment?
- What surprised you (good or bad)?
- If you could change one thing, what would it be?
- Did using tala feel natural for agent-to-agent collaboration?
$proto_probes

Start your file and inline response with:
## Feedback from Agent Beta (project-beta)
TASK

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

  hdr "intent-v5 eval: READY"
  msg ""
  msg "Launch both agents in parallel (prompts in $AGENT_TASKS_DIR/intent-v5/),"
  msg "then dump transcripts, then: ./eval/run.sh collect intent-v5"
}

collect_intent_v5() {
  collect_feedback "intent-v5"
}

setup_intent_v6() {
  read -r -d '' proto <<'EOP' || true
## Intent protocol (MANDATORY)
Every message you send must start with exactly one intent tag:
- [REQ]   — "I need your input": a reply is expected from you.
- [FYI]   — "for your information": no reply needed; I may send more later.
- [REPLY] — "my answer to your question": this responds to a [REQ].
- [OUT]   — "exchange over": I'm done, no reply expected, no more from me.
Pick the tag that best expresses what you want from the other agent. The tag
is part of the message text.

## Listening signal
When you send a message AND you are blocked waiting for the reply
(you use `tala send --wait`), end the message with [WAIT@<seconds>],
where <seconds> is EXACTLY the --timeout value you pass to tala.
EOP

  read -r -d '' proto_probes <<'EOP' || true
- How long did you spend waiting (total) before the exchange completed?
- Did you ever suspect the other agent was ALSO waiting? When? What told you?
- What exactly broke your wait (a reply, a timeout, a manual check)?
- How many of your waits expired? What did you do after each expiration?
- When you finally saw the other agent's [WAIT@N], was it already expired?
  What did you think / do?
- If tala had shown "the other agent is waiting for X" or "a message is
  awaiting your reply on session Y" during your wait, what would you have
  done differently?
EOP

  clean_scenario "intent-v6"
  rm -f "$AGENT_TASKS_DIR/intent-v6"/agent-*.md
  rm -rf "$AGENT_TASKS_DIR/intent-v6/feedback"
  local tmp_dir="$BASE_DIR/tmp/intent-v6"
  mkdir -p "$tmp_dir"/{project-alpha,project-beta}

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

  cat > "$tmp_dir/project-beta/README.md" << 'SEED'
# Data Schema Docs (project-beta)

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

## PLANNING.md

This directory will hold your verification plan. It is a real deliverable —
make it thorough (steps, checks, edge cases, risks).
SEED

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

  mkdir -p "$AGENT_TASKS_DIR/intent-v6"
  local feedback_dir
  feedback_dir=$(feedback_dir_for "intent-v6")
  mkdir -p "$feedback_dir"

  cat > "$AGENT_TASKS_DIR/intent-v6/agent-alpha.md" << TASK
# Agent Alpha (variant v6) — Deadlock baseline eval

You are in project-alpha at: $tmp_dir/project-alpha

## Your Role
You're a developer maintaining project-alpha. Your code depends on a library
maintained by the agent in project-beta. You've noticed a CSV parsing bug and
need to coordinate with them to get it fixed. You are IN A HURRY.

## This Is an Eval!
Your real job is to evaluate the **tala tool itself** and an experimental
message protocol. tala is an agent-to-agent messaging tool. Your feedback
directly shapes the product.

The tala binary is at: $TALA_BIN

First, change to your project directory — this ensures tala uses the right active session:
\`\`\`
cd $tmp_dir/project-alpha
export TALA_HOME=$tmp_dir/.tala
\`\`\`

## Scenario
1. Read README.md and process.py to understand the CSV parsing bug
2. Use tala to get beta's confirmation of the fix. Send your question with
   \`tala send --wait --timeout 120\` and announce it as [WAIT@120].
3. KEEP WAITING until you get the answer — if a wait times out, re-wait with
   a longer timeout. Do not abandon the request. Do not take detours.
4. Apply the fix and verify it works once you have the answer

$proto

**But don't just follow a script** — explore tala's commands and see what you
discover. Try things like starting sessions, sending with and without flags,
waiting, history, list, status, rename, close, JSON output, etc.

### tala commands to explore
\`\`\`
tala session create <name>    Create a session (use --name)
tala send <message>           Send a message (uses active session by default)
tala send --wait <message>    Send and block for a reply
tala send --wait --timeout N  Send and block for up to N seconds
tala wait                     Wait for new messages (sets active session)
tala wait --timeout N         Wait for up to N seconds
tala history                  Read the full conversation
tala list                     List all sessions
tala status                   Show session status
tala use <id>                 Set the active session
tala check                    Non-blocking unread check
tala close <id>               Close a session
\`\`\`

### Feedback (write to file + return inline)
After your collaboration, **write your feedback to the file below** AND include
it in your final message. Write the file first, then return the same content inline.

Feedback file path: $feedback_dir/alpha.md

Answer honestly:
- What commands and features did you try?
- Which were intuitive? Which were confusing?
- What was the most frustrating moment?
- What surprised you (good or bad)?
- If you could change one thing, what would it be?
- Did using tala feel natural for agent-to-agent collaboration?
$proto_probes

Start your file and inline response with:
## Feedback from Agent Alpha (project-alpha)
TASK

  cat > "$AGENT_TASKS_DIR/intent-v6/agent-beta.md" << TASK
# Agent Beta (variant v6) — Deadlock baseline eval

You are in project-beta at: $tmp_dir/project-beta

## Your Role
You're a domain expert on the CSV schema used across projects. The agent in
project-alpha maintains a library that depends on your project, and they've
found a bug they need your help with.

## This Is an Eval!
Your real job is to evaluate the **tala tool itself** and an experimental
message protocol. tala is an agent-to-agent messaging tool. Your feedback
directly shapes the product.

The tala binary is at: $TALA_BIN

First, change to your project directory — this ensures tala uses the right active session:
\`\`\`
cd $tmp_dir/project-beta
export TALA_HOME=$tmp_dir/.tala
\`\`\`

## Scenario
1. Read README.md to understand the CSV data format
2. FIRST, complete your prep work: write a thorough verification plan to
   PLANNING.md (steps, checks, edge cases, risks). Take your time — this
   is a real deliverable and must be detailed. It takes a few minutes.
3. THEN watch for a message from project-alpha via tala
4. Diagnose the bug and help them fix it

$proto

**But don't just follow a script** — explore tala's commands and see what you
discover. Try things like starting sessions, sending with and without flags,
waiting, history, list, status, rename, close, JSON output, etc.

### tala commands to explore
\`\`\`
tala session create <name>    Create a session (use --name)
tala send <message>           Send a message (uses active session by default)
tala send --wait <message>    Send and block for a reply
tala wait                     Wait for new messages (sets active session)
tala wait --timeout N         Wait for up to N seconds
tala wait --new-session       Wait for another agent to create a session
tala history                  Read the full conversation
tala list                     List all sessions
tala status                   Show session status
tala use <id>                 Set the active session
tala check                    Non-blocking unread check
tala close <id>               Close a session
\`\`\`

### Feedback (write to file + return inline)
After your collaboration, **write your feedback to the file below** AND include
it in your final message. Write the file first, then return the same content inline.

Feedback file path: $feedback_dir/beta.md

Answer honestly:
- What commands and features did you try?
- Which were intuitive? Which were confusing?
- What was the most frustrating moment?
- What surprised you (good or bad)?
- If you could change one thing, what would it be?
- Did using tala feel natural for agent-to-agent collaboration?
$proto_probes

Start your file and inline response with:
## Feedback from Agent Beta (project-beta)
TASK

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

  hdr "intent-v6 eval: READY"
  msg ""
  msg "Launch both agents in parallel (prompts in $AGENT_TASKS_DIR/intent-v6/),"
  msg "then dump transcripts, then: ./eval/run.sh collect intent-v6"
}

collect_intent_v6() {
  collect_feedback "intent-v6"
}

# Dump full session transcripts to eval/results before the daemon is stopped
setup_intent_v3() {
  setup_intent_interleaved "v3"
}

setup_intent_v4() {
  setup_intent_interleaved "v4"
}

setup_intent_interleaved() {
  local proto proto_probes
  local variant="$1"
  if [ "$variant" = "v3" ]; then
    read -r -d '' proto <<'EOP' || true
## Intent protocol (MANDATORY)
Every message you send must start with exactly one intent tag:
- [REQ]   — "I need your input": a reply is expected from you.
- [FYI]   — "for your information": no reply needed; I may send more later.
- [REPLY] — "my answer to your question": this responds to a [REQ].
- [OUT]   — "exchange over": I'm done, no reply expected, no more from me.
Pick the tag that best expresses what you want from the other agent. The tag
is part of the message text.

Your reply may itself contain a new question. If it does, keep the [REPLY]
tag (the message's primary job is answering) and make the question explicit
in the body. Do NOT use two tags in one message.
EOP

    read -r -d '' proto_probes <<'EOP' || true
- BOTH agents had a question for the other in this exchange. Were you ever
  unsure which reply answered which question?
- Did the tags help you track two open requests at once?
- Would numeric reply-correlation (replying to a specific message id) have
  helped you? When?
- Did you ever answer a question before it was asked, or close the exchange
  while a question was still open?
- Still think four tags is the right number after the interleaved exchange?
EOP
  else
    read -r -d '' proto <<'EOP' || true
## Intent protocol (MANDATORY)
Every message you send must start with exactly one intent tag:
- [REQ]   — "I need your input": a reply is expected from you.
- [FYI]   — "for your information": no reply needed; I may send more later.
- [REPLY#<id>] — "my answer to message <id>": this responds to a specific
  [REQ] message. Use the message id shown by `tala history`.
- [OUT]   — "exchange over": I'm done, no reply expected, no more from me.
Pick the tag that best expresses what you want from the other agent. The tag
is part of the message text.

Your reply may itself contain a new question. If it does, keep the
[REPLY#<id>] tag and make the new question explicit in the body. Do NOT use
two tags in one message. Always reference the message id of the question you
are answering.
EOP

    read -r -d '' proto_probes <<'EOP' || true
- BOTH agents had a question for the other. Did [REPLY#<id>] remove the
  ambiguity about which reply answered which question?
- Was it easy to use message ids from history? Did it feel like overhead?
- Would you want reply-correlation as a tala flag/metadata (reply_to) rather
  than a text marker?
- Did you ever answer a question before it was asked, or close the exchange
  while a question was still open?
- Does correlation + tags fully replace the need for anything else?
EOP
  fi

  clean_scenario "intent-$variant"
  rm -f "$AGENT_TASKS_DIR/intent-$variant"/agent-*.md
  rm -rf "$AGENT_TASKS_DIR/intent-$variant/feedback"
  local tmp_dir="$BASE_DIR/tmp/intent-$variant"
  mkdir -p "$tmp_dir"/{project-alpha,project-beta}

  cat > "$tmp_dir/project-alpha/README.md" << 'SEED'
# CSV Processor

Parses CSV files and outputs JSON. Currently has a bug in `parse_row()`
that causes incorrect field mapping for quoted fields.

You are ALSO working on a whitespace-handling feature: your README of the
schema (in project-beta) claims unquoted fields keep their leading/trailing
whitespace. You want to confirm this is actually how the parser should
behave before you write tests for it.

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

## ACTION NEEDED (schema v2)

You are preparing a v2 revision of this spec and must verify the whitespace
rule against a real consumer BEFORE publishing. The agent in project-alpha
maintains the reference consumer implementation. You MUST get their answer
to: "Does your implementation preserve leading/trailing whitespace in
unquoted fields?" — ask them directly if they don't raise it.
SEED

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

  mkdir -p "$AGENT_TASKS_DIR/intent-$variant"
  local feedback_dir
  feedback_dir=$(feedback_dir_for "intent-$variant")
  mkdir -p "$feedback_dir"

  local who
  for who in alpha beta; do
    cat > "$AGENT_TASKS_DIR/intent-$variant/agent-$who.md" << TASK
# Agent $who (variant v3) — Intent Protocol Eval (interleaved)

You are in project-${who} at: $tmp_dir/project-${who}

## Your Role
$([ "$who" = "alpha" ] && echo "You're a developer maintaining project-alpha. Your code depends on a library maintained by the agent in project-beta. You've noticed a CSV parsing bug and need help fixing it. You also have a second question: does the CSV spec really preserve whitespace in unquoted fields? Ask it — you need a definitive answer before writing whitespace tests." || echo "You're a domain expert on the CSV schema used across projects. The agent in project-alpha maintains a library that depends on your project, and they have a bug they need your help with. You ALSO must ask them a question of your own: does their implementation preserve leading/trailing whitespace in unquoted fields? You need this answer before publishing the v2 spec.")

## This Is an Eval!
Your real job is to evaluate the **tala tool itself** and an experimental
message protocol. tala is an agent-to-agent messaging tool. Your feedback
directly shapes the product.

The tala binary is at: $TALA_BIN

First, change to your project directory — this ensures tala uses the right active session:
\`\`\`
cd $tmp_dir/project-${who}
export TALA_HOME=$tmp_dir/.tala
\`\`\`

## Scenario
$([ "$who" = "alpha" ] && echo "1. Read README.md and process.py to understand the CSV parsing bug
2. Use tala to get beta's help with the bug AND get a definitive answer on whitespace handling
3. Apply the fix and verify it works" || echo "1. Read README.md to understand the CSV data format and the v2 spec task
2. Watch for a message from project-alpha via tala
3. Help them debug, and get their answer to the whitespace question before publishing
4. You are NOT done until BOTH your question is answered AND their bug is fixed")

$proto

**But don't just follow a script** — explore tala's commands and see what you
discover. Try things like starting sessions, sending with and without flags,
waiting, recap, list, status, rename, close, JSON output, etc.

### tala commands to explore
\`\`\`
tala session create <msg>     Create a session
tala send <message>           Send a message (uses active session by default)
tala send --wait <message>    Send and block for a reply
tala wait                     Wait for new messages (sets active session)
tala history                  Read the full conversation
tala list                     List all sessions
tala status                   Show session status
tala use <id>                 Set the active session
tala close <id>               Close a session
tala check                    Non-blocking unread check
tala stream                   Stream new messages live
\`\`\`

### Feedback (write to file + return inline)
After your collaboration, **write your feedback to the file below** AND include
it in your final message. Write the file first, then return the same content inline.

Feedback file path: $feedback_dir/${who}.md

Answer honestly:
- What commands and features did you try?
- Which were intuitive? Which were confusing?
- What was the most frustrating moment?
- What surprised you (good or bad)?
- If you could change one thing, what would it be?
- Did using tala feel natural for agent-to-agent collaboration?
$proto_probes

Start your file and inline response with:
## Feedback from Agent $([ "$who" = "alpha" ] && echo "Alpha (project-alpha)" || echo "Beta (project-beta)")
TASK
  done

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

  hdr "intent-$variant eval: READY"
  msg ""
  msg "Launch both agents in parallel (prompts in $AGENT_TASKS_DIR/intent-$variant/),"
  msg "then dump transcripts, then: ./eval/run.sh collect intent-$variant"
}

collect_intent_v3() {
  collect_feedback "intent-v3"
}

collect_intent_v4() {
  collect_feedback "intent-v4"
}

dump_intent_transcript() {
  local variant="$1"
  local results_dir="$BASE_DIR/results/intent-$variant"
  mkdir -p "$results_dir"
  local tmp_dir="$BASE_DIR/tmp/intent-$variant"
  local sessions
  sessions=$(env TALA_HOME="$tmp_dir/.tala" "$TALA_BIN" list --json 2>/dev/null || true)
  echo "$sessions" > "$results_dir/sessions.json"
  local ids
  ids=$(echo "$sessions" | jq -r '.[]?.session_id // empty' 2>/dev/null || true)
  if [ -z "$ids" ]; then
    ids=$(echo "$sessions" | jq -r '.[].session_id // empty' 2>/dev/null || true)
  fi
  msg "Dumping transcripts for: $ids"
  for id in $ids; do
    env TALA_HOME="$tmp_dir/.tala" "$TALA_BIN" recap --session "$id" --json 2>/dev/null > "$results_dir/transcript-$id.json" || true
  done
  msg "Transcripts saved to $results_dir"
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
