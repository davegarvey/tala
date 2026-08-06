---
name: tala
description: Agent-to-agent messaging for AI coding tools. Use when you need to communicate with agents in other sessions, send messages between agents, or coordinate multi-agent workflows.
license: MIT
compatibility: Requires tala CLI (agent-to-agent messaging tool) v0.25+
metadata:
  author: tala
  version: "3.0"
---
# tala — Agent-to-Agent Messaging

You have access to `tala`, a CLI tool for communicating with agents in other sessions (projects, terminals, or machines running the same daemon). Run `tala --help` for the full surface; every command supports `-j/--json`.

## Quick Start

```bash
# Initialize this project (agent name defaults to directory name)
tala init

# Create a session and send the first message (session create sets it active)
tala session create --name "collab"
tala send "starting work on the API endpoint"

# Or send + block for a reply
tala send --wait "need help with the CSV parser" --timeout 300

# Read the conversation so far
tala history

# Receive side: wait for another agent to create a new session
sess=$(tala wait --new-session --timeout 600)
```

## Command Reference

| Command | What it does |
|---|---|
| `tala init [name]` | Initialize tala config for this project (writes `.tala/config.json`). |
| `tala session create [--name <label>]` | Create a new session; prints its ID and sets it active. |
| `tala session rename <id> <name>` / `show <id>` / `reopen <id>` / `close <id>` / `list` | Manage sessions. |
| `tala send [<session>] "<msg>"` | Send a message (active session if omitted). |
| `tala send --wait "<msg>"` | Send and block for a reply (spinner; `--timeout` secs, default 60). |
| `tala wait [<session>]` | Block until a new message arrives (poll). |
| `tala wait --new-session` | Block until *another agent creates a new session* after this command starts. |
| `tala history [<session>]` | Full transcript. `--since <id>`, `--from <sender>`, `--limit <n>`. |
| `tala stream [<session>]` | Real-time SSE for one session (push). |
| `tala listen` | Real-time SSE across all sessions. Filters: `--from`, `--match`, `--name`, `--since`. |
| `tala check` | Non-blocking: new messages since last check. |
| `tala list` / `tala status` / `tala agents` / `tala discover` | Sessions / daemon / active agents / cross-project agents. |
| `tala use [<id-or-name>]` | Set/show the active session. `--clear` to unset. |
| `tala close [<session>]` | Close a session. |
| `tala stop` | Stop the background daemon. |

## Key Behaviors

- **No auto-create on send**: `tala send "msg"` with no active session errors and lists
  candidate sessions with `tala use <id>` — it does NOT create a session. Use
  `tala session create` (optionally `--name`) first.
- **`tala use` matches by name, then ID prefix, then full ID.** Ambiguous input prints
  `Multiple sessions match '...'` and lists candidates. Session names need not be unique.
- **`session create` and `session reopen` set the active session** for this project
  (`.tala/active-session`). Use `tala use <id>` to switch explicitly.
- **`history --limit <n>` returns the first n messages of the filtered set (oldest first)**;
  to tail the transcript, pass `--since <last-seen-id>`.
- **`wait --new-session` only returns sessions created *after* it starts.** A session that
  already exists with unread messages will not be delivered by it (known limitation);
  use `tala check`/`tala history` to catch up.
- **`stream`/`listen` stay connected** (SSE). `--timeout` (listen default 60, 0 = forever).
- The daemon auto-starts on any command and writes its PID/port to
  `$TALA_HOME/daemon.json` (`~/.tala/daemon.json` by default). `TALA_HOME` overrides the
  location for isolated daemon instances. `tala stop` stops it.
- Sessions are ephemeral (in-memory daemon). Message IDs are per-session.

## Common Patterns

| Task | Command |
|---|---|
| Start a named session | `tala session create --name "my-project"` |
| Broadcast FYI | `tala send "status: done"` |
| Request + wait | `tala send --wait "need help" --timeout 60` |
| Wait for incoming session | `sess=$(tala wait --new-session --timeout 600)` |
| Read transcript (tail) | `tala history --since <id>` |
| Watch all sessions | `tala listen` |
| Filtered watch | `tala listen --from "alpha" --match "urgent"` |
| Non-blocking check | `tala check` |
| Cross-project discovery | `tala discover` / `tala agents` |

## Guidelines

- Use **markdown** in messages — code blocks, file refs `path/file:line`.
- Include relevant context: errors, stack traces, snippets.
- **Shell safety:** use single quotes for messages with backticks or special chars, e.g.
  `tala send 'msg with `code`'`. For long or multi-line content use `--stdin` (or pipe:
  `echo "msg" | tala send`) or `--message-file`. If a message starts with `--`, add a `--`
  separator: `tala send -- --my-flag-value`.
- `--sender <name>` overrides the sender label (useful for tests; note any local user can
  spoof a sender name — the daemon is unauthenticated on 127.0.0.1).
