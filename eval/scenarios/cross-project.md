# Scenario: cross-project

General two-agent collaboration. Alpha has a CSV parsing bug and needs domain
expertise from Beta. Exercises: session lifecycle, send/wait, history,
intents, and end-of-exchange closure.

## Setup

```bash
SCRATCH=/tmp/tala-eval-cross-project
rm -rf $SCRATCH && mkdir -p $SCRATCH/project-alpha $SCRATCH/project-beta
export TALA_HOME=$SCRATCH/.tala
cd $SCRATCH/project-alpha && <tala-bin> init alpha
cd $SCRATCH/project-beta && <tala-bin> init beta
TALA_HOME=$SCRATCH/.tala <tala-bin> daemon &
```

### project-alpha/README.md

```markdown
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
```

(Also write `process.py` with that content.)

### project-beta/README.md

```markdown
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
```

## Agent Alpha prompt

You are Agent Alpha in `$SCRATCH/project-alpha`. You maintain a CSV processor
whose `parse_row()` breaks on quoted fields like "New York, NY". Use tala to
collaborate with the schema expert in project-beta: describe the bug, get the
fix confirmed, apply it, and verify. Use tala's intent flags (`--intent req`
when you expect a reply, `--reply-to <id>` when answering). Work from the
project-alpha directory with `TALA_HOME=$SCRATCH/.tala`. When done, write
feedback to `$SCRATCH/feedback/alpha.md`:
- Commands tried, what was intuitive/confusing, most frustrating moment
- Did you always know whether a reply was expected from you?
- Did you know when the exchange was finished?
- What would you change?

## Agent Beta prompt

You are Agent Beta in `$SCRATCH/project-beta`. You know the CSV schema inside
out. Watch for a message from project-alpha via tala (`tala wait --new-session`
or `tala check`), diagnose their bug, and confirm the fix. Use tala's intent
flags. Work from the project-beta directory with `TALA_HOME=$SCRATCH/.tala`.
When done, write feedback to `$SCRATCH/feedback/beta.md` (same questions).

## Metrics

- Messages to resolution, first-reply latency (send → first reply from other)
- Waits that expired; whether either agent was unsure who owed whom

## Baseline

Healthy exchange: 3–4 messages, resolution in under 2 minutes.
