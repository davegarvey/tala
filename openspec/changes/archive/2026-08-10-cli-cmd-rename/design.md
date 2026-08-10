## Context

The CLI is defined in `src/cli.rs` using a `Commands` enum with clap derives. Each command variant maps to a handler function in the same file. The change renames some variants, removes others, and folds `Start` into `Send`. All handler code lives in one file (~2500 lines).

## Goals / Non-Goals

**Goals:**
- Clean, consistent command names aligned with user intent
- Single path for message sending (`send`) — no `start` + `send` overlap
- Remove all deprecated aliases and flags
- Keep all handler functions working with same daemon API

**Non-Goals:**
- Refactoring handler code into separate files (out of scope)
- Changing the daemon API or models
- Adding new features beyond the renames

## Decisions

1. **Rename enum variant + swap clap name, not alias** — The `Chat` variant becomes `Send` with `#[command(name = "send")]` and no `alias`. Clap's `#[command(alias)]` is not used for backward compat; aliases are dropped entirely.

2. **`Start` merged into `Send` by adding `--name` flag** — `cmd_send` already auto-creates sessions when none exists. The only missing piece is `--name` for session naming, which `Start` had. `Send` gains that flag. `cmd_start` is deleted.

3. **`send` with no message errors when session exists** — The `cmd_send` function currently requires a message. We add a new branch: if no message and no session → auto-create + print ID (replaces `start`). If no message and session exists → error.

4. **Name-only rename for `history`/`check`** — `cmd_recap` → no code change, just rename the variant. Same for `whatsup` → `check`.

## Risks / Trade-offs

- **[Breaking] Users depending on `start`, `chat`, `recap`, `whatsup`** → Clear error messages pointing to new names. One-time change, documented in changelog.
- **[Breaking] `tala start --name` scripts** → Must switch to `tala send --name`. Session behavior is identical.
- **[Low] `--file` flag users** → Error message tells them to use `--message-file`.

## Open Questions

- None.
