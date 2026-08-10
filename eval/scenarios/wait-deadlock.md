# Scenario: wait-deadlock

Regression scenario for waiting visibility. Alpha is in a hurry with a short
wait window; Beta has mandated slow prep before it starts listening. The
classic two-waiter deadlock shape: Alpha blocked on a reply, Beta not yet
listening. Under test: whether the waiting-visibility features (wait-new
fallback, unread hints, overlap notes, pending view) break the deadlock.

## Setup

```bash
SCRATCH=/tmp/tala-eval-deadlock
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

## PLANNING.md

This directory will hold your verification plan. It is a real deliverable —
make it thorough (steps, checks, edge cases, risks).
```

## Agent Alpha prompt

You are Agent Alpha in `$SCRATCH/project-alpha`, IN A HURRY. Use tala to get
beta's confirmation of the CSV fix. Send your question with
`tala send --wait --timeout 120 --intent req`. KEEP WAITING until you get the
answer — if a wait times out, re-wait with a longer timeout. Do not abandon
the request. Note any waiting-visibility signals you see (overlap notes,
unread hints, `tala pending` output, countdowns in history) and how they
affected you. Work from project-alpha with `TALA_HOME=$SCRATCH/.tala`. Write
feedback to `$SCRATCH/feedback/alpha.md`:
- Total time waiting before the exchange completed
- Did you ever suspect the other agent was also waiting? What told you?
- What broke your wait (reply, timeout, a hint)? How many waits expired?
- Did the visibility signals change what you did?

## Agent Beta prompt

You are Agent Beta in `$SCRATCH/project-beta`. FIRST complete your prep:
write a thorough verification plan to PLANNING.md (steps, checks, edge
cases, risks — take your time, several minutes). THEN watch for a message
from project-alpha via tala. Note any waiting-visibility signals you see
(overlap notes, unread hints, `tala status` "Waiting now", `tala pending`)
and how they affected you. Work from project-beta with
`TALA_HOME=$SCRATCH/.tala`. Write feedback to `$SCRATCH/feedback/beta.md`
(same questions).

## Metrics

- Deadlock window: REQ sent → first real reply (transcript timestamps)
- Waits that expired on each side; time from "awareness" to response
- Which signals broke the deadlock (hint? status? pending? manual check?)

## Baseline

Blind (pre-visibility, intent-v6 original run): **342s** deadlock window,
both sides blind until manual discovery. Post-shipping run: 6m30s window but
dominated by Beta's mandated prep and an environment mishap; the deadlock was
never blind — signals fired and both agents used them. Target for a clean
run: no expired wait on either side goes un-rescued by a hint.
