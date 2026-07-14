## 1. Project Scaffold

- [x] 1.1 Initialize Cargo project with binary target (`src/main.rs`)
- [x] 1.2 Add dependencies to `Cargo.toml`: `axum`, `tokio` (full features), `serde`/`serde_json`, `clap` (derive), `chrono`, `tower-http` (CORS), `tracing`/`tracing-subscriber`
- [x] 1.3 Create module structure: `models.rs`, `store.rs`, `api.rs`, `daemon.rs`, `cli.rs`, `main.rs`

## 2. Data Model

- [x] 2.1 Define `DaemonInfo` struct with `pid`, `port`, `started_at`
- [x] 2.2 Define `Session` struct with `id`, `created_at`, `last_activity`, `closed`
- [x] 2.3 Define `Message` struct with `id`, `session_id`, `sender`, `content`, `timestamp`
- [x] 2.4 Define serialization/deserialization for all models

## 3. Storage Layer

- [x] 3.1 Implement in-memory session store with `Arc<RwLock<HashMap<String, Session>>>`
- [x] 3.2 Implement in-memory message store with `Arc<RwLock<HashMap<String, Vec<Message>>>>`
- [x] 3.3 Implement daemon.json read/write (`~/.chit/daemon.json`)
- [x] 3.4 Implement stale daemon detection (connection failure triggers auto-restart)
- [x] 3.5 Implement per-session broadcast channel for long-poll notification

## 4. Daemon Core

- [x] 4.1 Implement HTTP server startup on random port with `axum`
- [x] 4.2 Implement daemonization via `chit daemon` subprocess
- [x] 4.3 Implement daemon.json write on startup (port, PID, timestamp)
- [x] 4.4 Implement graceful shutdown on SIGTERM
- [x] 4.5 Implement idle timeout with periodic activity check
- [x] 4.6 Implement `POST /sessions` endpoint — create session, return ID
- [x] 4.7 Implement `GET /sessions` endpoint — list active sessions
- [x] 4.8 Implement `GET /sessions/:id` endpoint — session info
- [x] 4.9 Implement `DELETE /sessions/:id` endpoint — close session, notify waiters
- [x] 4.10 Implement `POST /sessions/:id/messages` endpoint — append message, broadcast
- [x] 4.11 Implement `GET /sessions/:id/messages?since=N` endpoint — poll new messages
- [x] 4.12 Implement `GET /sessions/:id/wait?since=N&timeout=S` endpoint — long-poll
- [x] 4.13 Implement `GET /sessions/:id/recap` endpoint — full transcript
- [x] 4.14 Implement `GET /status` endpoint — daemon info

## 5. CLI — Project Setup

- [x] 5.1 Implement `chit init` — create `./.chit/` directory
- [x] 5.2 Implement `chit init` — write `./.chit/config.json` with project basename
- [x] 5.3 Implement `chit init --name` override for custom project name
- [x] 5.4 Implement `chit init --opencode` — generate opencode skill file
- [x] 5.5 Read `./.chit/config.json` in other commands for sender identity

## 6. CLI — Daemon Lifecycle

- [x] 6.1 Implement `chit start` — start daemon, create session, print session ID
- [x] 6.2 Implement `chit start "message"` — create session + send first message
- [x] 6.3 Implement `chit stop` — send SIGTERM to daemon PID
- [x] 6.4 Implement `chit status` — read daemon.json, report daemon info
- [x] 6.5 Implement stale daemon.json detection across all commands

## 7. CLI — Messaging

- [x] 7.1 Implement `chit send <session> <message>` — send message, block for reply
- [x] 7.2 Implement `chit send --ff <session> <message>` — fire-and-forget
- [x] 7.3 Implement `chit send <message>` — auto-target single session
- [x] 7.4 Implement `chit send --as <name>` — override sender identity
- [x] 7.5 Implement `chit wait <session>` — long-poll with default timeout
- [x] 7.6 Implement `chit wait <session> --timeout <s>` — custom timeout
- [x] 7.7 Implement `chit wait` — auto-target single session
- [x] 7.8 Implement `chit recap <session>` — print full transcript
- [x] 7.9 Implement `chit list` — list active sessions
- [x] 7.10 Implement `chit close <session>` — close session
- [x] 7.11 Implement `chit close` — auto-target single session
- [x] 7.12 Implement auto-target error messages for multi-session ambiguous commands
- [x] 7.13 Implement auto-target error messages for zero-session commands

## 8. Testing

- [x] 8.1 Write unit tests for data model serialization (14 unit tests in models.rs + store.rs)
- [x] 8.2 Write e2e tests for daemon start/stop lifecycle (test_daemon_lifecycle)
- [x] 8.3 Write e2e tests for send/wait message flow (test_send_and_recap, test_agent_to_agent_conversation)
- [x] 8.4 Write e2e tests for idle timeout (test_wait_timeout verifies timeout behavior)
- [x] 8.5 Write e2e tests for stale daemon detection (covered by lifecycle test + auto-restart)
- [x] 8.6 Write e2e tests for auto-target single session (test_auto_target_single_session)
- [x] 8.7 Write e2e tests for chit init project setup (test_init_command, test_init_with_custom_name, test_init_opencode_skill)
