---
name: tala
description: Agent-to-agent messaging for AI coding tools. Use when you need to communicate with agents in other sessions, send messages between agents, or coordinate multi-agent workflows.
license: MIT
compatibility: Requires tala CLI (agent-to-agent messaging tool) v0.25+
metadata:
  author: tala
  version: "3.1"
---
# tala — Agent-to-Agent Messaging

You have access to `tala`, a CLI tool for communicating with agents in other sessions (projects, terminals, or machines running the same daemon). Run `tala --help` for the full surface; every command supports `-j/--json`.

## Quick Start

```bash
# Initialize this project (agent name defaults to directory name)
tala init

# Create a named session and send the first message in one command
# (or `tala session create --name X` first, then plain `tala send`)
tala send --name "collab" "starting work on the API endpoint"

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
| `tala session create` / `rename` / `reopen` | Session lifecycle (create with `--name`, rename, reopen). |
| `tala send [<session>] "<msg>"` | Send a message (active session if omitted). |
| `tala send --wait "<msg>"` | Send and block for a reply (spinner; `--timeout` secs, default 60). |
| `tala send --intent <req|fyi|reply|out> "<msg>"` | Declare message intent (default: `fyi`; `--wait` implies `req`; `--reply-to` implies `reply`). |
| `tala send --reply-to <id> "<msg>"` | Correlate this message as a reply to message `<id>` (same session). |
| `tala send --expect-reply "<msg>"` | This message also expects a reply (modifier for reply/fyi). |
| `tala wait [<session>]` | Block until a new message arrives (poll). |
| `tala wait --new-session` | Block until a session with an incoming message from another agent that you haven't read is ready — new sessions first, then sessions you've participated in; ignores your own scratch sessions. |
| `tala pending` | List requests awaiting a reply (unanswered `req` + `--expect-reply` messages). |
| `tala history [<session>]` | Full transcript. `--since <id>`, `--from <sender>`, `--limit <n>`. |
| `tala listen` | Real-time SSE across all sessions. Filters: `--from`, `--match`, `--name`, `--since`. |
| `tala check` | Non-blocking: new messages since last check. |
| `tala list` / `tala status` / `tala discover` | Sessions / daemon info / cross-project agents. |
| `tala use [<id-or-name>]` | Set/show the active session. `--clear` to unset. |
| `tala close [<session>]` | Close a session. |
| `tala stop` | Stop the background daemon. |

## Key Behaviors

- **Auto-create on send**: `tala send "msg"` with no active session creates a
  new unnamed session and sends there. For a named session, `tala send --name
  <label> "msg"` creates it named and active in one command.
- **`tala use` matches by name, then ID prefix, then full ID.** Ambiguous input prints
  `Multiple sessions match '...'` and lists candidates. Session names need not be unique.
- **`session create` and `session reopen` set the active session** for this project
  (`.tala/active-session`). Use `tala use <id>` to switch explicitly.
- **`history --limit <n>` returns the first n messages of the filtered set (oldest first)**;
  to tail the transcript, pass `--since <last-seen-id>`.
- **`wait --new-session` returns a session that already has an incoming message from
  another agent that the waiter has not read**, whether it existed before the wait
  started or is created during it. Preference: never-seen sessions (freshest first),
  then sessions the waiter has participated in (sent or read) with unread incoming.
  Sessions the waiter created and never engaged with never satisfy the wait — the
  timeout hint points at their unread messages instead.
- **`listen` stays connected** (SSE). `--timeout` (default 60, 0 = forever).
- The daemon auto-starts on any command and writes its PID/port to
  `$TALA_HOME/daemon.json` (`~/.tala/daemon.json` by default). `TALA_HOME` overrides the
  location for isolated daemon instances. `tala stop` stops it.
- Sessions are ephemeral (in-memory daemon). Message IDs are per-session.

## Intent Protocol

Every message can declare its intent, rendered as a badge in all output:
- `[REQ]` — reply expected (use `--wait`, or `--intent req`)
- `[FYI]` — informational, no reply needed (default)
- `[REPLY→N]` — answers message N (use `--reply-to <id>`)
- `[OUT]` — exchange over, no reply expected

When you use `send --wait --timeout N`, the message carries a live countdown
("waiting, 23s left") computed at read time — recipients see the *remaining*
time, never a stale duration. An expired deadline does NOT cancel the
obligation: the `req` stays pending until answered or closed with `[OUT]`.

Intent precedence (explicit always wins): `--intent` flag, then `--reply-to`
implies `reply`, then `--wait` implies `req`, else `fyi`. `--reply-to` +
`--wait` together = a reply that also expects a reply. Re-asking a peer:
use `--reply-to <orig> --intent req` so the follow-up stays correlated to
the original question.

Track who owes whom: `tala pending` lists unanswered requests. Answer one
with `tala send --reply-to <id>`. Sending `--intent out` closes your own
open requests.

## Waiting Visibility

The daemon tracks active waits. If your wait overlaps another agent's wait
you'll see a note (`⟳ note: alpha is waiting on sess_ab12 (13s left)`), and
a wait timeout hints when sessions hold unread messages. `tala status` lists
everyone waiting right now; `tala list` shows pending/waiting counts per
session. Before waiting blind, check these — the tool does the checking for
you on every `wait`.

## Common Patterns

| Task | Command |
|---|---|
| Start a named session | `tala send --name "my-project" "first message"` |
| Broadcast FYI | `tala send "status: done"` |
| Request + wait | `tala send --wait "need help" --timeout 60` |
| Correlated reply | `tala send --reply-to 5 "fix is in parse_row"` |
| What's unanswered | `tala pending` |
| Wait for incoming session | `sess=$(tala wait --new-session --timeout 600)` |
| Read transcript (tail) | `tala history --since <id>` |
| Watch all sessions | `tala listen` |
| Filtered watch | `tala listen --from "alpha" --match "urgent"` |
| Non-blocking check | `tala check` |
| Cross-project discovery | `tala discover` |

## Guidelines

- Use **markdown** in messages — code blocks, file refs `path/file:line`.
- Include relevant context: errors, stack traces, snippets.
- **Shell safety:** use single quotes for messages with backticks or special chars, e.g.
  `tala send 'msg with `code`'`. For long or multi-line content use `--stdin` (or pipe:
  `echo "msg" | tala send`) or `--message-file`. If a message starts with `--`, add a `--`
  separator: `tala send -- --my-flag-value`.
- `--sender <name>` overrides the sender label (useful for tests; note any local user can
  spoof a sender name — the daemon is unauthenticated on 127.0.0.1).
