## Context

The eval cross-project scenario identified three P1 issues and several P2 issues in tala's CLI. The codebase is a Rust CLI app using clap for argument parsing, axum for the daemon HTTP server, and tokio for async. Key files are `src/cli.rs` (command handlers), `src/api.rs` (daemon API), `src/models.rs` (data types), and `src/store.rs` (persistence).

## Goals / Non-Goals

**Goals:**
- Fix `--file -` to read from piped stdin
- Fix daemon status detection in `tala discover` to use port probing
- Fix `tala agents` to show agents from discovered projects and session participants without requiring prior messages
- Fix self-message exclusion from unread counters
- Add `--cursor` as alias for `--since` on recap
- Enhance `tala use` output with session name and message count

**Non-Goals:**
- Full refactoring of the daemon discovery architecture
- Renaming or consolidating `wait`/`stream`/`listen` commands (deferred)
- Documentation or help text improvements for `send`/`chat` aliases (purely editorial)

## Decisions

### Decision 1: `--file -` should delegate to the existing stdin reading path
Rather than reading stdin at the `--file` handling point, when `-` is detected as the filename, the code should set the `use_stdin` flag and fall through to the existing `--stdin` reading logic. This avoids duplicating the async stdin reading with timeout logic.

Alternatively considered: Reading stdin directly in the `--file` branch. Rejected because it duplicates the timeout and is_terminal logic already implemented for `--stdin`.

### Decision 2: Daemon status should use TCP port probe as fallback
For `tala discover`, if the daemon's `/api/agents` endpoint doesn't respond, attempt a TCP connection to the host:port from `daemon.json`. If the port is open, report "running" even if the API endpoint failed.

Alternatively considered: Probing `/api/health` or another endpoint. Rejected because any listening HTTP port implies the daemon is running; a specific health endpoint adds maintenance burden.

### Decision 3: Agent derivation should include session-level agent info beyond messages
`tala agents` currently derives agents only from message senders. To show agents before messaging, the agent list should also include agents discovered via project discovery (from `daemon.json` config). Each daemon knows its own agent identity, and this should be registered as a participant.

The approach: The `/api/agents` endpoint should also report the daemon's own agent from `config.json` and any agents recorded as session participants (not just message senders). Sessions track their participants, and this should be populated at session creation time.

Alternatively considered: Having the daemon register itself as an agent on startup. Rejected because sessions are the natural unit of agent association.

### Decision 4: Self-message exclusion via sender comparison
The unread count computation filters out messages where the sender matches the local agent name. The local agent name is read from `.tala/config.json`.

Alternatively considered: Adding a "read by sender" flag. Rejected as over-engineering for the simple case of excluding self-sent messages.

## Risks / Trade-offs

- [Risk] Port probing is not a definitive check (another process could be using the port) → Acceptable because the port is only used when `daemon.json` references it, and the daemon's port is ephemeral
- [Risk] Agent name from `config.json` might not match message sender names → This is an existing assumption; the same name source is used throughout
- [Risk] The self-message exclusion might conflict with multi-identity scenarios → Trade-off accepted; current model assumes one agent per project
