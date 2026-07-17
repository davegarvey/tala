---
name: chit
description: Agent-to-agent messaging for AI coding tools. Use when you need to communicate with agents in other sessions, send messages between agents, or coordinate multi-agent workflows.
license: MIT
compatibility: Requires chit CLI (agent-to-agent messaging tool)
metadata:
  author: chit
  version: "1.1"
---
# chit — Agent-to-Agent Messaging

You have access to `chit`, a CLI tool for communicating with agents in other sessions.

## Commands

- `chit start [message]` — Start a new session (optionally with initial message). Outputs a session ID like `sess_abc12`.
- `chit send [message]` — Send a message. Returns immediately by default. Use `-w`/`--wait` to block for a reply. Use `--session <id>` or set an active session with `chit use <id>`. Use `--file <path>` or `-` for stdin. Use `--as <name>` to override sender. Use `-q`/`--quiet` to suppress confirmation.
- `chit wait [session]` — Block until a new message arrives. Use `--timeout <secs>` to set a timeout. Use `--since <id>` for delta reads, `--from <sender>` to filter by sender, `--limit <n>` to cap results (0 = unlimited).
- `chit follow [session]` — Stream new messages as they arrive (SSE). Use `--since <id>` to catch up, `--timeout <secs>` to auto-disconnect, `--limit 0` for unlimited.
- `chit recap [session]` — View the full conversation transcript. Use `--since <id>` and `--limit <n>` for pagination (0 = unlimited).
- `chit close [session]` — Close a session.
- `chit observe` — Watch **all sessions** globally. Use `--channel <id>` to watch a specific session, `--match <pattern>` to filter messages, `--from <sender>` to filter by sender, `--since <id>` to replay from a point.
- `chit use <id>` — Set an active session for this project (saved per-directory). Use `chit use --clear` to unset.
- `chit list` — List all sessions.
- `chit init [name]` — Initialize chit in this project (creates `.chit/config.json`).

## Key Behaviors (v0.19+)

- **`chit send` returns immediately** by default (no-wait). Messages are fire-and-forget.
- To block for a reply, use `-w` or `--wait`. You'll see `⏎ Waiting for reply...` while waiting.
- Active session: use `chit use <id>` to avoid `--session` on every send. Saved per project directory.
- `--file <path>` reads message from a file; use `-` as message to read from stdin.

## JSON Output

All commands support `--json` for structured output. JSON responses include a `cursor` field with the last message ID — use with `--since` for pagination.

## Guidelines

- Format messages in **markdown** — use code blocks with language tags, file references as `path/file:line`, and links where useful.
- Include relevant context: error messages, file paths, stack traces, code snippets.
- When using `chit send` without `--wait`, the other agent must actively check for messages using `chit wait` or `chit recaps`.
