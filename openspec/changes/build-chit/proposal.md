## Why

AI coding agents (opencode, Claude Code, etc.) work in isolated sessions, each focused on a single project. When a bug or dependency issue crosses project boundaries — e.g., a library project like grubble breaks a consumer project like emendi — there's no way for the agents to communicate directly. The human must context-switch, relay information manually, or fall back to GitHub Issues as an awkward coordination layer.

A lightweight CLI tool for agent-to-agent messaging lets agents collaborate across sessions naturally, without the human becoming the message bus.

## What Changes

- **New CLI tool `chit`** written in Rust for agent-to-agent messaging
- **Background daemon** (`chit start`) that relays messages between sessions
- **Per-project identity** via `./.chit/config.json` (created by `chit init`)
- **Daemon discovery** via `~/.chit/daemon.json` — all CLI commands auto-locate the server
- **Long-poll blocking** for real-time message delivery (`chit wait`)
- **Markdown messages** with no enforced schema
- **Auto-generated session IDs** for ad-hoc use; human-friendly names later for persistent mode
- **Docker/persistent mode** planned but not in initial scope

## Capabilities

### New Capabilities
- `daemon`: Background process management — start/stop, idle timeout, stale PID detection, daemon discovery via `~/.chit/`
- `messaging`: Core send/wait/recap primitives — markdown messages, blocking reply on send, long-poll wait with timeout, fire-and-forget flag, full transcript recap
- `sessions`: Session lifecycle — auto-generated IDs (`sess_*`), auto-target single session, multiple session support with explicit targeting, close, list, status
- `project-setup`: `chit init` command — creates `./.chit/config.json` with project identity, optionally creates opencode skill

### Modified Capabilities
None (new project, no existing specs).

## Impact

- New Rust binary crate at the repository root
- Dependencies: `axum` (HTTP server), `tokio` (async runtime), `serde`/`serde_json` (serialization), `clap` (CLI parsing), `tower-http` (CORS)
- No changes to existing projects or systems
- Distribution: `cargo install chit`, Homebrew later
