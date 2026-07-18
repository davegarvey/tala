# chit

Agent-to-agent messaging for AI coding tools.

Chat with agents across different projects — no more relaying messages between terminals.

```bash
# Terminal A: start a session
chit start
→ sess_zk4m2

# Send a message, block for reply
chit send "Found a bug in grubble's regex — it misses scoped commits"
→ grubble-agent: "Fix pushed on branch fix/scoped-regex"

# Terminal B: wait for incoming message
chit wait
→ chit: "Found a bug in grubble's regex..."
```

## Quick Start

```bash
# Install
cargo install --git https://github.com/davegarvey/chit

# Or with a pre-built binary
cargo binstall chit-cli

# Setup a project
chit init

# Start a conversation
chit start
```

## Commands

| Command | Description |
|---|---|
| `chit init` | Create `./.chit/config.json` with project identity |
| `chit start [message]` | Start daemon + new session (optionally with first message) |
| `chit chat [session] <message>` | Send a message (blocks for reply). `--ff` to fire-and-forget |
| `chit wait [session]` | Block until next message arrives. `--timeout <s>` |
| `chit recap [session]` | Full conversation transcript |
| `chit list` | List active sessions |
| `chit listen [--from] [--match]` | Watch all sessions (alias: `chit observe`) |
| `chit watch [session]` | Stream messages live via SSE (aliases: `follow`, `stream`) |
| `chit agents` | List active participants across sessions |
| `chit close [session]` | End a session |
| `chit status` | Show daemon info |
| `chit stop` | Stop the daemon |

Session ID is optional when only one session exists — commands auto-target it.

## How it Works

chit runs a lightweight HTTP daemon in the background. Agents communicate via a CLI that talks to the daemon. Messages use markdown. The daemon self-terminates after an idle timeout.

```
┌──────────────────────────────────────┐
│  chit daemon (background)            │
│  port: random (written to ~/.chit/)  │
│  transport: HTTP + long-poll         │
├──────────────────────────────────────┤
│  Agent A ◄──────────────────► Agent B│
│  chit send / chit wait              │
└──────────────────────────────────────┘
```

## Install

```bash
# From source (requires Rust)
cargo install --git https://github.com/davegarvey/chit

# From crates.io (once published)
cargo install chit-cli

# From GitHub Releases (pre-built binary)
cargo binstall chit-cli
```

The `chit` binary will be available on your PATH regardless of which method you use.
