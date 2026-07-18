## Context

The cross-project eval critic identified 3 P1 and 3 P2 issues from two agents testing tala. This change addresses all 6. The codebase is Rust, using clap for CLI, reqwest for HTTP, and SSE for streaming.

## Goals / Non-Goals

**Goals:**
- Add `tala discover` command for finding agents in parent/sibling projects
- Add help-text cross-references for message-watching commands
- Rename `--new` to `--new-session` on `tala wait`
- Change `tala listen` default `--since` from 0 to cursor value (new messages only)
- Fix active session integrity on close and reopen
- Clarify command organization in help text

**Non-Goals:**
- No full peer-to-peer agent protocol (still single-daemon per project)
- No daemon-side changes for discovery (uses existing `/api/agents` endpoint)
- No structural reorganization of the CLI enum

## Decisions

### D1: Cross-project discovery — parent/sibling directory scan
- **Decision**: `tala discover` scans parent directories (up to 3 levels) for `.tala/config.json`, then scans siblings of each parent. For each discovered project, reads config (agent name) and daemon.json (host/port). If daemon reachable, queries `/api/agents`.
- **Implementation**: Pure CLI-side scan — no new daemon endpoints needed. Uses existing `store::tala_home()` logic pattern for reading configs.
- **File**: `cmd_discover()` in cli.rs + helper `discover_projects()` in store.rs or new module.

### D2: Help text cross-references
- **Decision**: Add `after_help` / `long_about` text to clap command definitions for `Wait`, `Listen`, `Stream`, `WhatsUp`, `Recap`, `Agents`.
- **File**: cli.rs — edit the `#[command(about = "...", long_about = "...")]` attributes.

### D3: --new to --new-session rename
- **Decision**: Change the clap attribute `long = "new"` to `long = "new-session"`. Add `alias = "new"` for backward compatibility.
- **File**: cli.rs — edit the `r#new: bool` field on `Wait`.

### D4: Listen default since from cursor
- **Decision**: In `cmd_listen`, replace `since.unwrap_or(0)` with `since.unwrap_or_else(|| store::read_cursor().await.unwrap_or(0))`. Update cursor on message receive.
- **File**: cli.rs — `cmd_listen` function.

### D5: Active session on close/reopen
- **Decision**: In `cmd_close`, when session is sourced from `resolve_session_id` (active session implicit), clear the active session file after successful close and print a warning about the cleared session.
- **Decision**: In `cmd_session_reopen`, after successful reopen, write the session as active and print "(now active)".
- **File**: cli.rs — `cmd_close` and `cmd_session_reopen`.

### D6: Command organization hints
- **Decision**: Add `after_help` to `Use` mentioning `tala session`. Add aliases/notes to `SessionCommands::List` and `SessionCommands::Close` help.
- **File**: cli.rs.

## Risks / Trade-offs

- [Directory scanning is imprecise] — scanning parent/sibling dirs is best-effort; it may miss projects in arbitrary locations. Acceptable for a discovery aid.
- [Listen default change is behavioral] — existing users relying on `tala listen` showing full history will be surprised. Mitigated by documented `--since 0` opt-in.
- [--new alias compatibility] — keeping `--new` as a hidden alias ensures existing scripts and skills continue working.
- [Close clearing active session] — when user does `tala close` without args, they probably want to close the active session and have it cleared. Explicit close with `tala close sess_id` preserves the active session.
