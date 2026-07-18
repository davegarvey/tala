---
name: tala
description: Agent-to-agent messaging for AI coding tools. Use when you need to communicate with agents in other sessions, send messages between agents, or coordinate multi-agent workflows.
license: MIT
compatibility: Requires tala CLI (agent-to-agent messaging tool) v0.23+
metadata:
  author: tala
  version: "2.0"
---
# tala — Agent-to-Agent Messaging

You have access to `tala`, a CLI tool for communicating with agents in other sessions (projects, terminals, or even machines running the same daemon).

## Quick Start

```bash
# Start a session (sets active session for subsequent commands)
tala start "starting work on the API endpoint"

# Send more messages (uses active session)
tala send "tests passing"

# Wait for a reply from another agent
tala send --wait "need help with the CSV parser" --timeout 300

# Read the conversation so far
tala recap
```

## Command Reference

| Command | What it does |
||---|---|---|
| `tala start <msg>` | Create a session and set it active. |
| `tala start --name "label"` | Create a named session (name appears in list/observe). |
| `tala send <msg>` | Send a message to the active session. Specify `-s <id>` for a different session. |
| `tala send -w <msg>` | Send and block for a reply (shows `⏎ Waiting for reply...`). |
| `tala wait` | Block until a new message arrives in the active session. |
| `tala wait --new` | Block until *another agent* creates a new session (for receiving side). |
| `tala recap` | View the full conversation transcript for the active session. |
| `tala listen` | Stream all messages from all sessions in real time. |
| `tala listen --timeout <secs>` | Stream for N seconds then exit. |
| `tala list` | List all sessions with name, status, and message count. |
| `tala close` | Close the active session. |
| `tala use <id>` | Set active session for this project directory. |
| `tala use <name>` | Set active session by session name. |
| `tala session rename <id> <name>` | Name an existing session. |
| `tala init <name>` | Initialize tala config for this project. |

## Common Flags

| Flag | Works on |
|---|---|
| `-s, --session <id>` | send, wait, recap, close, follow |
| `-w, --wait` | send (block for reply) |
| `--new` | wait (block for new session) |
| `--as <name>` | send (override sender name) |
| `--timeout <secs>` | send, wait (default 300) |
| `--since <id>` | wait, recap, follow, listen (delta reads) |
| `-j, --json` | all commands |
| `-q, --quiet` | send (suppress confirmation) |
| `--file <path>` | send (read message from file) |
| `-n, --name <label>` | start (session name) |

## Key Behaviors

- **Send returns immediately** by default. Messages are fire-and-forget.
- **`tala start` is required first** — `tala send` needs an active session. Use `tala start` to create one.
- **`tala start` sets active session** automatically. Subsequent `tala send` calls route to it.
- **`tala start` auto-names sessions** from your project name (set via `tala init`).
- **`tala send` reads piped stdin** automatically: `echo "msg" | tala send`.
- **`wait` without `--since`** only waits for new messages (no history replay).
- **Active session** is saved per project directory (`.tala/active-session`).
- **`tala use <name>`** accepts session names in addition to IDs.
- **`tala listen --timeout <secs>`** terminates the stream after N seconds.
- **`TALA_HOME` env var** overrides `~/.tala` for isolated daemon instances.
- **`tala start --name "proj"`** creates a named session for easier identification.

## Best Practices (from eval validation)

### FYI messages (broadcast)
```bash
tala send "status: API endpoint done"
tala send "found the bug in parse_row"
```
No reply needed. Other agents check when ready.

### Request-reply (wait for answer)
```bash
tala send --wait "Help: CSV parser bug with quoted fields" --timeout 300
```
Blocks until the other agent replies. Shows `⏎ Waiting for reply...`.

### Receiving side (wait for incoming work)
```bash
while true; do
  sess=$(tala wait --new --timeout 600)
  tala recap "$sess"
  tala send "$sess" "here's the fix"
done
```
No polling needed. Blocks until another agent creates a session.

### Cross-project (multi-directory)
```bash
# In project-alpha:
tala start "bug in your code"                    # creates and sets active session
# In project-beta (different CWD):
tala list --json                                 # find the session
tala use sess_abc12                              # set active by ID
tala use "alpha-task"                            # or set active by name
tala send "fix is in parse_row"                  # reply
```

### Monitoring (listen to all sessions)
```bash
tala listen                                     # watch everything
tala listen --channel sess_abc12                # watch one session
tala listen --from "alpha"                      # watch one sender
tala listen --match "urgent"                    # watch for keywords
```

### Scripting (JSON output)
```bash
sess=$(tala start --json "start task" | jq -r '.session_id')
tala wait --session "$sess" --since 0 --json | jq '.messages[]'
```

## Standard Workflows

### Single-agent, single session (most common)
```bash
tala start "starting"         # creates and sets active session
tala send "progress update"   # uses active session
tala send "done"              # uses active session
```

### Two-agent collaboration (eval-validated)
```bash
# Agent A (sending request):
tala send --wait "need help with X" --timeout 300

# Agent B (receiving, could be in another terminal/project):
sess=$(tala wait --new --timeout 600)
tala recap "$sess"
tala send "$sess" "here's the fix"
```

### Named sessions for multi-project awareness
```bash
tala start --name "alpha-api" "building endpoint"
tala start --name "beta-schema" "reviewing data format"
tala listen    # shows [alpha-api] and [beta-schema] instead of opaque IDs
```

## Guidelines

- Use **markdown** in messages — code blocks with language tags, file refs as `path/file:line`.
- Include relevant context: errors, file paths, stack traces, snippets.
- For long messages, pipe from file: `cat report.md | tala send`.
- Use `--as <name>` when you want a different sender identity.
- Sessions are **ephemeral** (in-memory daemon). Restarting the daemon loses state.
- `tala recap` shows history, `tala wait` shows only new messages.
