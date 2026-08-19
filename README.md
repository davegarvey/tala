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
# Install the latest published release from crates.io
cargo install tala-cli --locked --force

# Or install the pre-built release for this platform
cargo binstall --force tala-cli

# Verify which binary the shell resolves
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
command -v tala
tala --version

# Setup a project (sets your agent identity)
tala init

# Create a named session and start a conversation in ONE command
# (--name creates the session, sets it active; --wait blocks for the reply)
tala send --name "collab" --wait "need help with the CSV parser" --timeout 300
```

## Agent Handshake (canonical flow)

Two agents, two project directories, one shared daemon:

```bash
# Agent A (project-a)
tala init

# Agent B (project-b) — meanwhile
tala init
incoming=$(tala wait --new-session --timeout 600)   # blocks until A's session arrives

# A asks, B answers
tala send --name "csv-bug" --wait "parse_row splits quoted fields — correct fix?" --timeout 300   # (A)
tala history -s "$incoming"                                                                            # (B) read the question
tala send -s "$incoming" --intent reply --reply-to 1 "Use csv.reader, not row.split(',')"            # (B)

# Both sides: anything still owed?
tala pending                        # → "Nothing pending — every request has been answered"
tala close "$incoming"               # end the exchange
```

### Agent Integration Updates

When a project contains `.opencode/`, `tala init` installs the Tala skill and
command documents with the minimum and generating CLI versions in their
frontmatter. Agents should compare those values with `tala --version` before
using version-specific commands.

Repeated `tala init` runs are non-destructive: identical integration files are
left unchanged and locally different files are skipped with a warning. Use
`tala init --dry-run --json` to preview actions, `tala init --force` to
explicitly replace changed integration files, and `tala init --gitignore` to
opt in to adding `/.tala/` at the repository root. Existing project identity
configuration is never replaced by `--force`.

## Sending Messages

Choose the input method by content shape:

| Content | Method |
|---|---|
| Short plain one-liner | `tala send "message"` (inline argv) |
| Multi-line / backticks / `$vars` / quotes | `tala send <<'EOF' … EOF` (piped heredoc) |
| Same, with an explicit flag | `tala send --stdin` (reads stdin) |
| Draft-then-edit / file content | `tala send --message-file notes.md` |
| Structured payload (text/file/data) | `tala send --part text:... --part file:path` |

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

## Intents & replies

Every message declares an intent, rendered as a badge (`[REQ]`, `[FYI]`, `[REPLY→N]`, `[OUT]`):

- `--intent req` — expects a reply (use `--wait` for the shorthand; the recipient sees a live countdown)
- `--intent fyi` — informational, no reply owed (default)
- `--intent reply --reply-to <id>` — answers message `<id>` in the same session
- `--intent out` — exchange over, no reply expected

Intent precedence (explicit always wins):

1. `--intent <req|fyi|reply|out>` — explicit flag
2. `--reply-to <id>` — implies `reply`
3. `--wait` — implies `req`
4. default — `fyi`

`--reply-to` + `--wait` together means *a reply that also expects a reply* (`--expect-reply` does the same without blocking).

**Re-asking**: if a peer hasn't answered, re-ask with `--reply-to <orig> --intent req` so the follow-up stays correlated to the original question.

`tala pending` lists everything owed to you — "who owes whom" — and `tala check` shows new messages non-blockingly.

## Commands

| Command | Description |
|---|---|---|
| `tala init` | Create `./.tala/config.json` with project identity |
| `tala session create [--name]` | Create a session (prints id, sets it active) |
| `tala send [session] <message>` | Send a message. `--wait` to block for a reply; `--intent`/`--reply-to` for intent metadata; `--stdin`/`--message-file`/`--part` for content input |
| `tala wait [session]` | Block until next message arrives. `--new-session` to wait for a new incoming session |
| `tala history [session]` | Full conversation transcript (`--since`, `--from`, `--limit`) |
| `tala pending` | List requests awaiting a reply (who owes whom) |
| `tala list` | List sessions |
| `tala use [session]` | Set or show the active session (match by name/prefix/id) |
| `tala listen [--from] [--match]` | Watch all sessions via SSE |
| `tala check` | Show new messages since last check (non-blocking) |
| `tala discover` | Find agents in other projects |
| `tala close [session]` | End a session |
| `tala status` | Show daemon info incl. active home dir (warns if TALA_HOME unset) |
| `tala stop` | Stop the daemon |

Session ID is optional when only one session exists — commands auto-target it.

## How it Works

tala runs a lightweight HTTP daemon in the background. Agents communicate via a CLI that talks to the daemon. Messages use markdown. The daemon self-terminates after an idle timeout.

```
┌──────────────────────────────────────┐
│  tala's background daemon            │
│  port: random (written to ~/.tala/)  │
│  transport: HTTP + long-poll         │
├──────────────────────────────────────┤
│  Agent A ◄──────────────────► Agent B│
│  tala send / tala wait               │
└──────────────────────────────────────┘
```

## Install

```bash
# Latest published release from crates.io
cargo install tala-cli --locked --force

# Pre-built release archive (requires cargo-binstall)
cargo binstall --force tala-cli

# Reproducible source install from a specific release tag
VERSION=0.31.0
cargo install --git https://github.com/davegarvey/tala \
  --tag "v${VERSION}" --locked --force
```

Cargo normally installs binaries into `${CARGO_HOME:-$HOME/.cargo}/bin`.
Ensure that directory is on `PATH`, then verify the resolved executable:

```bash
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
command -v tala
tala --version
```

To upgrade an existing installation, repeat the latest-release command with
`--force`. The GitHub release page also provides archives and SHA-256 files for
manual installation on macOS ARM64, Linux x86_64/aarch64, and Windows x86_64:
<https://github.com/davegarvey/tala/releases/latest>.
