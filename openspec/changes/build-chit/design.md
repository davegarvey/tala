## Context

chit is a new CLI tool enabling AI coding agents in separate sessions to communicate directly. Currently, cross-project issues (e.g., grubble breaks a consumer) require the human to relay information manually or use GitHub Issues as an awkward coordination layer.

The project is an empty Rust scaffold with OpenSpec configured. No existing code.

## Goals / Non-Goals

**Goals:**
- CLI tool for agent-to-agent messaging via a background daemon
- Ad-hoc sessions with auto-generated IDs; zero-config for single-session use
- Blocking send that waits for reply (agent-friendly turn-taking)
- Long-poll wait for real-time message delivery
- Per-project identity via `./.chit/config.json`
- Self-cleaning daemon lifecycle (idle timeout, stale PID detection)
- Markdown message format with no enforced schema
- `chit init` for project onboarding and opencode skill generation

**Non-Goals:**
- Cross-machine support (future)
- Docker/persistent mode (future)
- Authentication, encryption, TLS (local-only for now)
- WebSocket transport (long-poll is sufficient)
- Structured message schemas beyond markdown
- File attachments or binary content
- Message editing or deletion

## Decisions

### Language: Rust
- Why: Single binary distribution (`cargo install chit`), no runtime deps, matches grubble ecosystem, excellent async with tokio + axum.
- Alternatives: TypeScript/Node.js (runtime dependency), Go (not in user's stack).

### Transport: HTTP + long-poll
- Why: Simplest implementation that meets the "blocking wait" requirement. Works with any HTTP client (agents can use curl). WebSocket adds complexity without benefit for request-response messaging.
- How: Client sends `GET /sessions/:id/wait?since=<last_msg_id>&timeout=<s>`. Server holds connection until new message arrives or timeout expires.

### Daemon: Singleton per machine, multi-session
- Why: One daemon manages all ad-hoc sessions on a machine. Single port, simple discovery. Justifies the `~/.chit/` home directory.
- Discovery: Daemon writes `~/.chit/daemon.json` with `{ pid, port, started_at }`. All CLI commands read this to find the server.

### Daemon lifecycle: Foreground fork, idle timeout, stale detection
- `chit start` daemonizes to background. Writes PID file.
- Idle timeout (default 10min with no messages on any session) triggers graceful shutdown.
- If daemon.json points to a dead PID, next `chit` command detects staleness, cleans up, and auto-restarts if needed.
- Explicit `chit stop` sends SIGTERM to daemon.

### Session IDs: Auto-generated (`sess_*`)
- Why: Ad-hoc sessions need no human naming. Random 5-char alphanumeric slug suffices. Human-friendly names deferred to persistent/Docker mode.

### Send blocks for reply by default
- `chit send "message"` sends and then long-polls for the next message (same session). This matches the agent workflow: ask, get answer, continue.
- `--ff` / `--fire-and-forget` flag returns immediately after send.

### Identity: Project basename from `./.chit/config.json`
- `chit init` creates `./.chit/config.json` with `{ name: "<project-basename>" }`
- Override per-message with `--as <name>`
- Displayed in recap: `grubble: "fixed in analyser.rs"`

### Message storage: In-memory with optional persistence
- Ad-hoc mode: Messages held in daemon memory. Lost when daemon stops.
- Each message gets a monotonic integer ID for `since` tracking.
- Persistent mode (future): SQLite or JSON file in `~/.chit/sessions/<id>/`.

### API endpoints

```
POST   /sessions                    → create session
GET    /sessions                    → list sessions
GET    /sessions/:id                → session info
DELETE /sessions/:id                → close session

POST   /sessions/:id/messages       → send message
GET    /sessions/:id/messages       → poll new messages (?since=N)
GET    /sessions/:id/wait           → long-poll (?since=N&timeout=S)
GET    /sessions/:id/recap          → full transcript

GET    /status                      → daemon info
```

### Data model

```rust
struct DaemonInfo {
    pid: u32,
    port: u16,
    started_at: chrono::DateTime<chrono::Utc>,
}

struct Session {
    id: String,            // "sess_zk4m2"
    created_at: chrono::DateTime<chrono::Utc>,
    last_activity: chrono::DateTime<chrono::Utc>,
    closed: bool,
}

struct Message {
    id: u64,               // monotonic per session
    session_id: String,
    sender: String,        // project name or --as override
    content: String,       // markdown
    timestamp: chrono::DateTime<chrono::Utc>,
}
```

### CLI command mapping

| CLI | API | Notes |
|---|---|---|
| `chit start [message]` | `POST /sessions` + optional `POST /sessions/:id/messages` | Auto-detects single session for send/wait |
| `chit send [session] <msg>` | `POST /sessions/:id/messages` then `GET .../wait` | Blocking reply; `--ff` skips wait |
| `chit wait [session]` | `GET /sessions/:id/wait` | Optional `--timeout` |
| `chit recap [session]` | `GET /sessions/:id/recap` | |
| `chit list` | `GET /sessions` | |
| `chit close [session]` | `DELETE /sessions/:id` | Other waiters receive "session closed" |
| `chit status` | `GET /status` | |
| `chit stop` | kills daemon process | SIGTERM via PID file |
| `chit init` | none (filesystem only) | Creates `./.chit/config.json`, optional skill |

## Risks / Trade-offs

- **[In-memory storage]** Messages lost on daemon restart. Mitigation: Acceptable for ad-hoc; persistent mode later.
- **[Long-poll vs WebSocket]** Long-poll is simpler but less efficient under high concurrency. Mitigation: For 2-3 agents per session, efficiency is irrelevant.
- **[Single daemon]** If daemon crashes, all sessions are lost. Mitigation: Stale detection + auto-restart on next CLI command.
- **[No auth]** Localhost-only binding means any process on the machine can read/write sessions. Mitigation: Acceptable for developer tooling; Docker mode would add auth.
- **[Project basename collision]** Two projects named "client" in different directories would share identity. Mitigation: User can override with `chit init --name` or `--as`.
