## Scenario

Two agents working on separate projects need to coordinate. Agent Alpha is fixing a bug in a Python script but lacks domain expertise on the data format. Agent Beta has deep knowledge of the data spec. They must use tala to diagnose and fix the bug together.

## Setup

- `project-alpha/` — A Python project with a buggy CSV parser; a `README.md` describes the bug
- `project-beta/` — A companion project with the data schema docs; agent Beta's job is to help debug

## Agent Tasks

### Agent Alpha (project-alpha)

You are working in `{{ALPHA_DIR}}`. Review the project README, then use tala to send messages to the agent in the other project. Describe the bug clearly. When the other agent responds, apply their fix. Once the fix is verified, return feedback inline as part of your final Task message.

### Agent Beta (project-beta)

You are working in `{{BETA_DIR}}`. Review the project README. You know the data schema inside out. When an agent from the other project contacts you via tala, help them debug the issue. Point them to the exact line and fix. Once resolved, return feedback inline as part of your final Task message.

## Feedback

Each agent writes feedback to `$AGENT_TASKS_DIR/<scenario>/feedback/<agent>.md`
AND returns it inline in their Task result. The file is the source of truth for
the critique step; the inline copy is for the human reader.

Questions each agent answers:
- How easy was it to start using tala?
- How intuitive were send, wait, recap?
- Was there any confusion about the API (flags, defaults, session management)?
- What was the most frustrating part?
- What would you change or improve?
- Did the tool help or hinder collaboration?

## Eval Loop (updated)

```
1. Setup   →  ./eval/run.sh setup cross-project
2. Launch  →  Copy prompts into parallel Task tool calls
3. Collect →  ./eval/run.sh collect cross-project
               Reads saved feedback files, stops daemon
4. Critique → ./eval/run.sh critique cross-project
               Auto-injects saved feedback into critic prompt
5. Fix
6. Re-eval
```

## Seed Files

### project-alpha/README.md

```markdown
# CSV Processor (project-alpha)

Parses CSV files and outputs JSON. Currently has a bug in `parse_row()`
that causes incorrect field mapping for quoted fields.

## File: process.py

```python
import csv
import json
import sys

def parse_row(row):
    # BUG: doesn't handle quoted fields with commas
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
```

### project-beta/README.md

```markdown
# Data Schema Docs (project-beta)

Documents the CSV schema used across projects.

## CSV Format Rules

- All fields are separated by commas
- Fields containing commas, newlines, or double-quotes must be wrapped in double-quotes
- A double-quote character inside a quoted field is escaped with another double-quote
- Fields may have leading/trailing whitespace, which should be preserved unless quoted

## Valid Parsing Approach

Use Python's `csv.reader` or equivalent — it handles all quoting rules correctly.
The bug in project-alpha is that `parse_row` does `row.split(',')` instead of
using the `csv` module's reader properly. The fix is to remove `parse_row` entirely
and use `csv.reader` for the actual parsing, only converting to dict afterwards.
```

## Recommended tala workflow

1. Alpha starts a session: `tala start "need help with CSV parsing bug"`
2. Alpha sends detailed bug description: `tala send --session <id> "row.split(',') breaks on quoted fields like 'New York, NY'"`
3. Beta receives via `tala wait --session <id>` or `tala recap --session <id>` then replies with the fix
4. Alpha reads the fix, applies it, and confirms
5. Both agents write feedback to their respective files (file paths are in the task prompt)
