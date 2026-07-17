## Scenario

Two agents working on separate projects need to coordinate. Agent Alpha is fixing a bug in a Python script but lacks domain expertise on the data format. Agent Beta has deep knowledge of the data spec. They must use chit to diagnose and fix the bug together.

## Setup

- `project-alpha/` — A Python project with a buggy CSV parser; a `README.md` describes the bug
- `project-beta/` — A companion project with the data schema docs; agent Beta's job is to help debug

## Agent Tasks

### Agent Alpha (project-alpha)

You are working in `{{ALPHA_DIR}}`. Review the project README, then use chit to send messages to the agent in the other project. Describe the bug clearly. When the other agent responds, apply their fix. Once the fix is verified, write feedback to `{{RESULTS_FILE}}`.

### Agent Beta (project-beta)

You are working in `{{BETA_DIR}}`. Review the project README. You know the data schema inside out. When an agent from the other project contacts you via chit, help them debug the issue. Point them to the exact line and fix. Once resolved, write feedback to `{{RESULTS_FILE}}`.

## Feedback

Each agent writes feedback to the results file answering:
- How easy was it to start using chit?
- How intuitive were send, wait, recap?
- Was there any confusion about the API (flags, defaults, session management)?
- What was the most frustrating part?
- What would you change or improve?
- Did the tool help or hinder collaboration?

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

## Recommended chit workflow

1. Alpha starts a session: `chit start "need help with CSV parsing bug"`
2. Alpha sends detailed bug description: `chit send --session <id> "row.split(',') breaks on quoted fields like 'New York, NY'"`
3. Beta receives via `chit wait --session <id>` or `chit recap --session <id>` then replies with the fix
4. Alpha reads the fix, applies it, and confirms
