## Why

The cross-project eval revealed 7 actionable UX issues across the CLI: `tala wait --new-session` lacks initial feedback making the first experience feel broken; `tala listen` has no `--new-only` flag causing history replay noise; `tala use` without arguments silently says "no active session" instead of listing available sessions; the naming of `listen`/`stream`/`wait` is the top reported confusion for new users; `tala wait --new` is hard to discover because it's only in subcommand help; `unread_count` in `tala list --json` doesn't update the cursor after sending, so `tala list` can show stale counts; and the `tala wait --new` help mention needs to be at the top-level. Three P2 items are deferred: "agents shows no active agents" (design issue), "discover vs agents naming" (would need breaking rename), and "per-project active session confusion" (known design trade-off).

## What Changes

1. **`tala wait --new` initial feedback**: Print "Waiting for a new session (timeout: Ns)..." before initiating the long poll, so users get immediate confirmation.
2. **`tala listen` help documents `--since`**: A dedicated `--new-only` flag is deferred (requires server-side changes); instead the help text now documents `--since <n>` as the way to skip history replay.
3. **`tala use` session listing**: When no argument is given and no active session is set, list available sessions instead of a terse "No active session" message.
4. **CLI help clarity**: Improve help text for `wait`, `stream`, `listen` to cross-reference each other with clear "when to use" guidance.
5. **`tala wait --new` discoverability**: Mention `--new-session` in the wait command's top-level doc comment (shown in `tala --help`).
6. **Cursor update on send**: Add `write_cursor()` call in `cmd_send()` so `tala list` and `tala whatsup` reflect sent messages and don't show stale unread counts.
7. **`tala wait` initial feedback**: The regular `cmd_wait()` already shows initial feedback; `cmd_wait_new()` now also shows it.

## Capabilities

### New Capabilities
- *(none — `--new-only` deferred to future loop with server-side support)*

### Modified Capabilities
- `tala wait --new-session`: Now prints initial feedback with timeout
- `tala use` (no args): Now lists sessions when none active
- `tala send`: Now updates the cursor file after sending
- Help text for `wait`, `stream`, `listen`, `use`: Clarified cross-references

## Impact

- `src/cli.rs`: `cmd_send()`, `cmd_wait_new()`, `cmd_use()`, `cmd_listen()`, CLI help strings for multiple commands, `Listen` struct args
- No API or model changes required
