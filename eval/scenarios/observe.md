## Scenario

Three agents work on independent tasks in separate projects. A fourth agent (the monitor) watches all activity via `chit observe`. The monitor evaluates whether observe provides enough context to understand what's happening across the system.

## Setup

- `project-alpha/` — Building an API endpoint
- `project-beta/` — Building a CLI tool
- `project-gamma/` — Writing documentation
- The monitor runs from the root dir and uses `chit observe` to watch all sessions

## Agent Tasks (launch all in parallel)

### Agent Alpha (project-alpha)

You are building an API endpoint in `{{ALPHA_DIR}}`. Look at the README and implement the endpoint. Use chit to send status updates as you work (e.g., "starting implementation", "tests passing", "done"). Return feedback inline as part of your final Task message when done.

### Agent Beta (project-beta)

You are building a CLI tool in `{{BETA_DIR}}`. Look at the README and implement it. Send chit updates about your progress. Return feedback inline as part of your final Task message when done.

### Agent Gamma (project-gamma)

You are writing docs in `{{GAMMA_DIR}}`. Look at the README and write documentation. Send chit updates. Return feedback inline as part of your final Task message when done.

### Monitor (launch alongside the workers)

You are watching all agent activity via `chit observe`. Run `chit observe` from `{{MONITOR_DIR}}` — start it at the same time as the workers so you see live activity. Note whether you can follow all three conversations. Then return feedback inline as part of your final Task message answering the evaluation questions.

## Feedback (all agents)

- How easy was it to get started with chit?
- How intuitive were the commands you used?
- Was anything confusing or surprising?
- What would you improve?
- For the monitor specifically: did `chit observe` give you an accurate picture of what was happening?

## Seed Files

Each project has a minimal README with a small, self-contained task that should take ~2 minutes.

### project-alpha/README.md

```markdown
# API Service

Build a simple health-check endpoint. Create `src/server.py`:

```python
def handle_health():
    return {"status": "ok", "version": "1.0.0"}

if __name__ == "__main__":
    import json
    print(json.dumps(handle_health()))
```

Run with `python src/server.py` — should print `{"status": "ok", "version": "1.0.0"}`.
```

### project-beta/README.md

```markdown
# CLI Tool

Build a file watcher that prints file changes. Create `src/watch.py`:

```python
import sys

def watch(path):
    print(f"watching: {path}", file=sys.stderr)
    return {"watching": path}

if __name__ == "__main__":
    import sys
    result = watch(sys.argv[1] if len(sys.argv) > 1 else ".")
    print(result)
```

### project-gamma/README.md

```markdown
# Documentation

Write a simple README for the project explaining what it does.
Create `README.md` with at least a title, description, and usage section.
The project is "ChitChat" — a fictional messaging API.
```
