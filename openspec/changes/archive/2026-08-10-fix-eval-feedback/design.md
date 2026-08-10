## Context

Five fixes from the cross-project eval critic. All live in `src/cli.rs` — no API, model, or store changes needed. The daemon already exists and cursor functions (`read_cursor`, `write_cursor`) are already implemented in `src/store.rs`.

## Goals / Non-Goals

**Goals:**
- `tala start --wait` creates a session and blocks until the first reply arrives
- `tala wait` persists cursor after receiving messages so `tala whatsup` shows correct new-message state
- `tala init --json` outputs structured JSON (mirroring the pattern used by other commands)
- `tala init` starts the daemon automatically so setup is a single step
- Command alias (`send`/`chat`) is discoverable from help text

**Non-Goals:**
- No API changes (daemon endpoints stay the same)
- No data model changes
- No new subcommands

## Decisions

1. **start --wait implementation**: Reuse the existing wait logic from `cmd_send` (lines 1159-1207) which calls `/api/sessions/{id}/wait`. After creating the session, call the wait endpoint with the first message's ID as `since`. Share the wait logic by extracting it into a helper or duplicating the pattern cleanly.

2. **Cursor update in wait**: After `cmd_wait` receives messages (line 1407-1417), add `store::write_cursor(max_msg_id)` using the max message ID from `result.messages`. This mirrors the pattern in `cmd_send` (line 1143) and `cmd_listen` (line 1624).

3. **init --json flag**: Add `#[arg(long, short = 'j')] json: bool` to the Init command variant. In `cmd_init`, branch output based on `json_output`. Implicit start of daemon does not need JSON output since it's a silent side-effect.

4. **init daemon auto-start**: At the end of `cmd_init`, call `ensure_daemon_running().await?` to lazily start the daemon. This is already idempotent if the daemon is already running.

5. **Help alias visibility**: Add a note to the Chat command's `after_help` string: "Alias: tala send". Clap's `#[command(alias = "send")]` doesn't surface it in help text; inline `after_help` is the simplest fix.

## Risks / Trade-offs

- [start --wait changes command signature] → Adding `wait`/`timeout`/`json` params to `cmd_start` increases its argument count. Using a struct or keeping focused is fine; clippy already allows `too_many_arguments` on similar functions.
- [init daemon start could fail] → `ensure_daemon_running` already handles start failures gracefully. If the daemon can't start, init has already written config, so partial setup is clean.
- [Cursor update in wait changes global state] → The cursor is already a global monotonic counter updated by send, listen, and whatsup. Adding wait is consistent.
