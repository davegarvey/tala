---
name: chit
description: Agent-to-agent messaging for AI coding tools. Use when you need to communicate with agents in other sessions, send messages between agents, or coordinate multi-agent workflows.
license: MIT
compatibility: Requires chit CLI (agent-to-agent messaging tool) v0.23+
metadata:
  author: chit
  version: "2.0"
---
# chit — Agent-to-Agent Messaging

You have access to `chit`, a CLI tool for communicating with agents in other sessions (projects, terminals, or even machines running the same daemon).

## Quick Start

```bash
# Send a message (auto-creates session, returns immediately)
chit send "starting work on the API endpoint"

# Send more messages (uses active session)
chit send "tests passing"

# Wait for a reply from another agent
chit send --wait "need help with the CSV parser" --timeout 300

# Read the conversation so far
chit recap
```

## Command Reference

| Command | What it does |
|---|---|
| `chit send <msg>` | Send a message. Auto-creates session if none exists. Returns immediately. |
| `chit send -w <msg>` | Send and block for a reply (shows `⏎ Waiting for reply...`). |
| `chit wait` | Block until a new message arrives in the active session. |
| `chit wait --new` | Block until *another agent* creates a new session (for receiving side). |
| `chit recap` | View the full conversation transcript for the active session. |
| `chit observe` | Stream all messages from all sessions in real time. |
| `chit start --name "label"` | Create a named session (name appears in list/observe). |
| `chit list` | List all sessions with status and message count. |
| `chit close` | Close the active session. |
| `chit use <id>` | Set active session for this project directory. |
| `chit session rename <id> <name>` | Name an existing session. |
| `chit init <name>` | Initialize chit config for this project. |

## Common Flags

| Flag | Works on |
|---|---|
| `-s, --session <id>` | send, wait, recap, close, follow |
| `-w, --wait` | send (block for reply) |
| `--new` | wait (block for new session) |
| `--as <name>` | send (override sender name) |
| `--timeout <secs>` | send, wait (default 300) |
| `--since <id>` | wait, recap, follow, observe (delta reads) |
| `-j, --json` | all commands |
| `-q, --quiet` | send (suppress confirmation) |
| `--file <path>` | send (read message from file) |
| `-n, --name <label>` | start (session name) |

## Key Behaviors (v0.23+)

- **Send returns immediately** by default. Messages are fire-and-forget.
- **No `chit start` needed** — `chit send "msg"` auto-creates a session if none is active.
- **`chit send` reads piped stdin** automatically: `echo "msg" | chit send`.
- **`wait` without `--since`** only waits for new messages (no history replay).
- **Active session** is auto-set after `chit send` or `chit start`. Saved per project directory (`.chit/active-session`).
- **`CHIT_HOME` env var** overrides `~/.chit` for isolated daemon instances.
- **`chit start --name "proj"`** creates a named session for easier identification.

## Best Practices (from eval validation)

### FYI messages (broadcast)
```bash
chit send "status: API endpoint done"
chit send "found the bug in parse_row"
```
No reply needed. Other agents check when ready.

### Request-reply (wait for answer)
```bash
chit send --wait "Help: CSV parser bug with quoted fields" --timeout 300
```
Blocks until the other agent replies. Shows `⏎ Waiting for reply...`.

### Receiving side (wait for incoming work)
```bash
while true; do
  sess=$(chit wait --new --timeout 600)
  chit recap "$sess"
  chit send "$sess" "here's the fix"
done
```
No polling needed. Blocks until another agent creates a session.

### Cross-project (multi-directory)
```bash
# In project-alpha:
chit send "bug in your code"                    # session auto-created
# In project-beta (different CWD):
chit list --json                                 # find the session
chit use sess_abc12                              # set active
chit send "fix is in parse_row"                  # reply
```

### Monitoring (observe all sessions)
```bash
chit observe                                     # watch everything
chit observe --channel sess_abc12                # watch one session
chit observe --from "alpha"                      # watch one sender
chit observe --match "urgent"                    # watch for keywords
```

### Scripting (JSON output)
```bash
sess=$(chit send --json "start task" | jq -r '.session_id')
chit wait --session "$sess" --since 0 --json | jq '.messages[]'
```

## Standard Workflows

### Single-agent, single session (most common)
```bash
chit send "starting"          # auto-creates session
chit send "progress update"   # uses active session
chit send "done"              # uses active session
```

### Two-agent collaboration (eval-validated)
```bash
# Agent A (sending request):
chit send --wait "need help with X" --timeout 300

# Agent B (receiving, could be in another terminal/project):
sess=$(chit wait --new --timeout 600)
chit recap "$sess"
chit send "$sess" "here's the fix"
```

### Named sessions for multi-project awareness
```bash
chit start --name "alpha-api" "building endpoint"
chit start --name "beta-schema" "reviewing data format"
chit observe    # shows [alpha-api] and [beta-schema] instead of opaque IDs
```

## Guidelines

- Use **markdown** in messages — code blocks with language tags, file refs as `path/file:line`.
- Include relevant context: errors, file paths, stack traces, snippets.
- For long messages, pipe from file: `cat report.md | chit send`.
- Use `--as <name>` when you want a different sender identity.
- Sessions are **ephemeral** (in-memory daemon). Restarting the daemon loses state.
- `chit recap` shows history, `chit wait` shows only new messages.
