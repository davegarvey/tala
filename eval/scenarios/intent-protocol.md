# Scenario: intent-protocol

Two agents with interleaved questions — both ask AND answer. Exercises the
intent metadata end to end: `--intent` tags, `--reply-to` correlation, the
`pending` view, and deadline rendering.

## Setup

```bash
SCRATCH=/tmp/tala-eval-intent
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

You are ALSO working on a whitespace-handling feature: your reading of the
schema (in project-beta) claims unquoted fields keep their leading/trailing
whitespace. You want this confirmed before writing tests for it.

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

## ACTION NEEDED (schema v2)

You are preparing a v2 revision of this spec and must verify the whitespace
rule against a real consumer BEFORE publishing. The agent in project-alpha
maintains the reference consumer implementation. You MUST get their answer
to: "Does your implementation preserve leading/trailing whitespace in
unquoted fields?" — ask them directly if they don't raise it.
```

## Agent Alpha prompt

You are Agent Alpha in `$SCRATCH/project-alpha`. You have TWO things to get
from the schema expert in project-beta: (1) confirmation of the CSV fix for
`parse_row()`, and (2) a definitive ruling on whether unquoted-field
whitespace is preserved per the spec. You also need to answer whatever they
ask you. Use tala's intent flags properly:
- `--intent req` (or `--wait`) when you expect a reply
- `--intent reply --reply-to <id>` when answering
- Check `tala pending` before closing — you are NOT done while any of your
  requests are unanswered, and you must not close while their question to you
  is open
Work from project-alpha with `TALA_HOME=$SCRATCH/.tala`. Write feedback to
`$SCRATCH/feedback/alpha.md`:
- Were you ever unsure which reply answered which question?
- Did `tala pending` reflect reality? Did the countdowns render sensibly?
- Most frustrating moment; one thing you'd change

## Agent Beta prompt

You are Agent Beta in `$SCRATCH/project-beta`. You must answer alpha's CSV
questions AND get their answer to the whitespace question before publishing
schema v2. Use tala's intent flags. Check `tala pending` before closing —
NOT done until both your question is answered and their bug is fixed. Work
from project-beta with `TALA_HOME=$SCRATCH/.tala`. Write feedback to
`$SCRATCH/feedback/beta.md` (same questions).

## Metrics

- Both questions answered on each side; zero "closed while question open"
- `tala pending` accuracy at the end (should be empty)
- Reply correlation correctness under interleaving

## Baseline

Healthy: 4–6 messages, both questions answered, pending empty at close.
