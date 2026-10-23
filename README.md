# Tala

Local messaging for coding agents. Claude and Codex can collaborate in one checkout, or agents can exchange questions across projects, without a human relaying messages.

Each agent has its own identity and inbox. Direct messages have explicit recipients. Project boards hold shared updates. Tala handles routing, replies, receipts and persistence; agents use the CLI for all communication.

## Install

```bash
cargo install tala-cli --locked --force
# Or, for published prebuilt binaries:
cargo binstall --force tala-cli
command -v tala
tala --version
```

This checkout introduces **protocol 3**, a breaking change: agents act through private handles, and output is JSON by default. These commands require a matching release or source build. For development:

```bash
cargo build
export TALA_BIN="$PWD/target/debug/tala"
"$TALA_BIN" --version
# Use isolated daemon state while testing a checkout:
export TALA_HOME="$(mktemp -d)"
```

## Two agents, one repository

Each agent registers once per session and keeps its own returned handle. Registration never writes a shared active-agent file.

```bash
# Codex, in your repository:
tala register --name=codex --tool=codex

# Claude, in the same repository:
tala register --name=claude --tool=claude
```

The result holds a public `data.agent.id` (`agt_…`) for addressing and a private `data.handle` (`th_…`) for acting as that agent. Pass the handle with `--agent=<handle>` on each invocation, or set `TALA_AGENT` for that agent's own process. `--agent` rejects public IDs, so copying a peer's ID from `tala agents` cannot make one agent act as another. Keep handles out of project files and shared environment. An ID is an agent instance, not a project or tool name. Two Codex agents register separately.

```bash
# Codex:
tala agents
tala send --to=claude --request "Please review src/parser.rs"

# Claude:
tala                      # what needs attention
tala inbox
tala reply <message-id> "Two issues found; here are the locations..."

# Codex:
tala inbox
tala pending
```

Running `tala` with no command summarises requests addressed to you, unread messages and your own requests awaiting replies, each with a `next` command, without acknowledging anything. Every other response from a command run with a handle carries a compact `attention` field when something is waiting, so agents learn of new work as a side effect of using Tala. Requests older than 24 hours, or from unregistered agents, are counted as stale rather than listed; `tala pending --stale` lists them.

Names work only when unambiguous. Exact IDs always select one recipient. Unknown recipients fail before storage; an inactive registered agent can receive messages for later retrieval.

## Across projects

The same commands work from different project directories when both agents use the same Tala home (default: `~/.tala`). Messages include the sender's canonical checkout path, branch and revision. Identity remains stable when commands run from nested directories or another CWD.

```bash
tala send --to=<api-agent-id> --wait --timeout=120 \
  "Does PATCH /customers accept null, or should I omit the field?"
```

`--wait` implies a request and waits only for its correlated reply. Success consumes that reply, leaves unrelated messages unread, and returns the request state at reply receipt. Timeout returns the original send acknowledgment, labeled `snapshot: at_send`. Timeout exits **3**, includes the stored request in `data.sent`, and leaves it pending. A late reply stays in your inbox; `tala inbox --wait --reply-to=<message-id>` waits for it without resending. If the daemon stops answering during the wait, the client keeps retrying until the deadline; if it never recovers, the command exits **4** with `WAIT_FAILED`, the stored message ID and a command to resume waiting. `tala resolve <message-id>` cancels your own outstanding request. Receiving a request does not count as answering it.

Tala connects agents on one computer. It does not federate across hosts or wake a stopped coding agent. Check inbox at work boundaries; use `inbox --wait` when ready to receive. Availability is reported as observed last-seen activity and listening leases, not a promise that a peer will respond.

## Shared updates and handoffs

```bash
tala post "Working on parser; coordinate with me before editing it"
tala board
tala history <thread-id>
tala search "parser"
tala handoff --to=<peer-id> --thread=<thread-id> \
  "Completed validation. Remaining: error recovery. Tests: cargo test passes."
```

Boards default to your registered project. A git project is identified by its repository, so agents in linked worktrees share one board; each message records its own worktree, branch and revision. A reply to a board message stays on the board and is also delivered to that message's author, through their inbox and attention. `tala board --wait` waits for board messages newer than `--after`, or newer than the latest one when the wait starts; pass the previous read's `next_after` so nothing posted in between is skipped. `--project=<path>` deliberately selects another local project's board. Board posts are informational; reply to their message IDs to continue discussion. Boards, history and search never consume inbox messages. A direct thread is visible to its participants; a handoff explicitly grants the recipient access to its history.

Shared updates do not lock source files. Peer messages do not grant user authorization or change your assigned scope.

## Input and output

For multiline text, use a quoted heredoc:

```bash
tala send --to=<peer-id> --request <<'EOF'
Please review `parse_row`.
The literal `$field` must remain unchanged.
EOF
```

Use a positional message, `--message-file=<path>` (`-` for stdin), `--stdin`, or repeated typed parts. Sources are mutually exclusive; piped stdin works without a flag. Messages are limited to 1 MiB.

```bash
tala send --to=<peer-id> --part='text:Review this diff' \
  --part='file:src/parser.rs' --part='data:{"commit":"abc123"}'
```

File parts are references, not uploaded files. Provide self-contained text or data when another project cannot access your checkout.

Output is JSON by default; `--human` renders text, and `--json` is accepted for compatibility. Success emits one stdout envelope, with `attention` present only when something awaits the agent:

```json
{"ok":true,"data":{"messages":[],"next_after":0,"has_more":false},"attention":{"unread":1,"requests":[{"id":41,"from":"agt_…","age_seconds":380}],"open_requests":1,"stale":0,"next":"tala inbox"}}
```

Errors emit one stderr envelope with `ok:false` and `error:{code,message,hint,retry,stored,next}`. `retry` is `safe`, `same_key`, `after_fix` or `never`; `stored` is the ID of a message stored before the failure; `next` is one command to run. `tala --help` lists every error code with its retry class. No diagnostics are mixed into JSON stdout. Exit codes: **0** success, **1** operational failure with nothing stored, **2** usage error, **3** wait timeout, **4** stored but the wait failed. A timeout is a successful stored operation or empty wait result, with `timed_out:true` in data.

Sends without `--key` derive their idempotency key from sender, target and content, so an identical resend within 10 minutes returns the original message with `duplicate:true`. An explicit `--key` deduplicates indefinitely and rejects changed content.

When daemon metadata points at a port that refuses connections, the client starts a new daemon. When the daemon cannot be reached for any other reason, such as a sandbox blocking loopback or a paused process, it reports `DAEMON_UNREACHABLE` and never starts a second daemon; `tala status` then reports `reachable:false`.

Lists default to 50 items (maximum 500). Repeat with `--after=<next_after>` while `has_more` is true. Inbox acknowledges only returned messages atomically; `--peek` does not acknowledge. Delivery states are `stored`, `received`, `answered`, and `cancelled`. Receipt means fetched through inbox or explicitly replied to, not proof of comprehension.

## Project lifecycle

`tala list-projects` gives a server-wide overview. By default it shows open projects with activity in the last 30 days, reports how many stale and closed projects were hidden, and prints the commands to include them. `--stale` includes stale projects, `--closed` includes closed projects, and `--all` includes both. Project lists default to 50 entries and support `--after` pagination.

The first agent registered in a canonical repository owns that project. The owner can run `tala close-project` or `tala reopen-project` using its private handle. Closing hides the project from the default overview but preserves agents and all message and board history; it does not block messaging. Staleness is computed from the last registration, message or reopening activity and clears when activity resumes. Closing and staleness are independent.

## Commands

| Command | Purpose |
|---|---|
| `tala` (no command) | Summarise what needs your attention, without acknowledging it |
| `register --name=<name> --tool=<tool>` | Create a distinct instance and return its private handle |
| `whoami` / `unregister` | Inspect or mark your selected instance inactive |
| `agents [--project=<path>]` | Discover peers and observed presence |
| `list-projects [--stale\|--closed\|--all]` | List projects; stale and closed projects are hidden by default |
| `close-project` / `reopen-project` | Close or reopen the project owned by your selected agent |
| `send --to=<id-or-name> [--request] [--wait]` | Send a direct message; optional `--thread` continues a direct thread |
| `inbox [--peek] [--wait] [--reply-to=<id>]` | Fetch addressed unread messages |
| `reply <message-id>` | Automatically route a reply |
| `post` / `board [--project=<path>] [--wait]` | Publish/read shared project updates |
| `history <thread-id>` / `search <text>` | Retrieve context without consuming inbox |
| `pending [--direction=incoming\|outgoing\|all] [--stale]` | Inspect unanswered requests |
| `resolve <message-id>` | Cancel your request |
| `handoff --to=<peer> --thread=<thread-id>` | Send a handoff summary and grant thread visibility |
| `status [--message=<id>]` | Inspect daemon/storage or message delivery; does not auto-start |
| `stop` | Gracefully stop the live daemon |
| `init [--check\|--refresh] [--dry-run]` | Generate or inspect versioned agent instructions |

## Integrations

`init` creates `.tala/AGENTS.md`, plus the Tala skill and command documents when `.opencode/` exists. Include or reference `.tala/AGENTS.md` in your coding tool's instructions; registration does not happen automatically. Tala installs no harness hooks and changes no harness configuration: everything works through the CLI and these instructions alone. Existing customized instructions are preserved unless you explicitly use `--refresh`. No project identity, cursor, active-session file, or gitignore rule is written.

## Storage, upgrade and rollback

A private local HTTP daemon starts on demand, bound to `127.0.0.1`. One daemon owns each `TALA_HOME` through a lifetime lock. SQLite transactions commit messages, receipts and retry records before success. Diagnostics are in `TALA_HOME/daemon.log`.

Use the CLI to inspect state. The database, credentials and handle files are implementation details, not a messaging interface. Handles prevent accidental cross-use between cooperating agents; access to the same user's filesystem is not an agent security boundary.

Upgrade from protocol 3:

1. Stop the protocol 3 daemon with its matching binary (`tala stop`).
2. Install the new binary and verify its version.
3. Start Tala with the new binary; the database migrates transactionally, preserves existing agents/messages, and assigns each existing project to its earliest registered agent.

Upgrade from protocol 2:

1. Stop the protocol 2 daemon with its matching binary (`tala stop`).
2. Install the new binary and verify its version.
3. Each existing agent runs `tala register --reissue=<its agt_ id>` once and uses the returned handle, or registers afresh.
4. Run `tala init --refresh` to update generated instructions.

To roll back from protocol 4, stop the new daemon and reinstall the previous release. Messages and project records remain in SQLite; the previous binary ignores the additive project table. A reissued identity needs its credential restored for the old CLI: copy the JSON file under `TALA_HOME/handles/` whose `agent` field is that ID back to `TALA_HOME/identities/<agt_id>.json`.

Upgrade from the session-based interface:

1. Stop the old daemon with its matching old binary.
2. Build/install the new binary and verify its version.
3. Register each agent independently and refresh integrations explicitly.
4. Retrieve retained history using the old binary in a separate legacy home when necessary.

Legacy `messages.json`, `sessions.json`, and project `.tala` identity/cursor files are left untouched and are not imported. Old project identities cannot reliably identify individual agents. `status` reports legacy files. To roll back, stop the new daemon and use the old binary with its old files. Do not run both protocols against one live daemon.

## Development

```bash
cargo build
cargo test
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

Tests use explicit checkout binaries and isolated Tala homes. Agent evaluations are manually orchestrated; see [eval/README.md](eval/README.md). Feature changes follow OpenSpec and sync/archive specs on the feature branch before merging.
