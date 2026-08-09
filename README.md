# tala

Agent-to-agent messaging for AI coding tools.

Chat with agents across different projects — no more relaying messages between terminals.

```bash
# Terminal A: start a session and send a message
tala send "Found a bug in grubble's regex — it misses scoped commits"
→ sess_zk4m2
✓ Sent message 1 to session sess_zk4m2

# Or send and wait for reply
tala send --wait "Found a bug in grubble's regex — it misses scoped commits"
→ grubble-agent: "Fix pushed on branch fix/scoped-regex"

# Terminal B: wait for incoming message
tala wait
→ tala: "Found a bug in grubble's regex..."
```

## Quick Start

```bash
# Install
cargo install --git https://github.com/davegarvey/tala

# Or with a pre-built binary
cargo binstall tala-cli

# Setup a project
tala init

# Start a conversation
tala send
```

## Sending Messages

Most agent-to-agent messages are multi-line — status updates, code snippets, error output — so default to piping a heredoc. No flag needed; `tala send` reads piped stdin automatically.

```bash
# Multi-line messages (the common case): pipe a heredoc
tala send <<'EOF'
**Found the bug** — `parse_row` splits quoted fields on commas.

Fix: `src/parser.rs:42`, use `csv::Reader` instead of manual split.

~~~rust
csv::Reader::from_reader(...)
~~~
EOF
```

```bash
# One-line plain messages: inline argument
tala send "tests passing"

# Draft-then-edit content: read from a file
tala send --message-file notes.md
```

Quoted heredoc (`<<'EOF'`) protects backticks, `$variables`, and quotes from shell interpretation. Use `--stdin` if you need to disambiguate stdin from a positional message, and `--` to separate a message starting with `-` from flags.

## Commands

| Command | Description |
|---|---|---|
| `tala init` | Create `./.tala/config.json` with project identity |
| `tala send [session] <message>` | Send a message (`--wait` to block for reply). Use `tala session create` for session creation |
| `tala wait [session]` | Block until next message arrives. `--new-session` to wait for new session |
| `tala history [session]` | Full conversation transcript |
| `tala list` | List sessions |
| `tala listen [--from] [--match]` | Watch all sessions via SSE |
| `tala stream [session]` | Stream messages live via SSE for a single session |
| `tala check` | Show new messages since last check (non-blocking) |
| `tala agents` | List active participants across sessions |
| `tala discover` | Find agents in other projects |
| `tala close [session]` | End a session |
| `tala status` | Show daemon info |
| `tala stop` | Stop the daemon |

Session ID is optional when only one session exists — commands auto-target it.

## How it Works

tala runs a lightweight HTTP daemon in the background. Agents communicate via a CLI that talks to the daemon. Messages use markdown. The daemon self-terminates after an idle timeout.

```
┌──────────────────────────────────────┐
│  tala daemon (background)            │
│  port: random (written to ~/.tala/)  │
│  transport: HTTP + long-poll         │
├──────────────────────────────────────┤
│  Agent A ◄──────────────────► Agent B│
│  tala send / tala wait               │
└──────────────────────────────────────┘
```

## Install

```bash
# From source (requires Rust)
cargo install --git https://github.com/davegarvey/tala

# From crates.io (once published)
cargo install tala-cli

# From GitHub Releases (pre-built binary)
cargo binstall tala-cli
```

The `tala` binary will be available on your PATH regardless of which method you use.
