## Context

Seven UX fixes from the cross-project eval loop, all in the CLI handler layer (`src/cli.rs`). No API, model, or store changes needed. The fixes are ergonomic: better feedback, discoverability, and cursor correctness.

## Goals / Non-Goals

**Goals:**
- `tala wait --new-session` prints "Waiting for a new session (timeout: Ns)..." immediately
- `tala listen --new-only` skips historical message replay
- `tala use` with no args lists sessions when no active session is set
- Help text for `wait`/`stream`/`listen` cross-references each other
- `tala wait --new-session` mentioned in top-level `tala wait --help`
- `cmd_send()` writes cursor so `tala list` shows correct unread counts
- Regular `tala wait` feedback already present but verified

**Non-Goals:**
- No API changes (daemon endpoints stay the same)
- No data model changes
- No new CLI subcommands (only new flags)

## Decisions

1. **`wait --new` feedback**: In `cmd_wait_new()`, print "Waiting for a new session (timeout: {timeout}s)..." to stderr before the HTTP call. Matches the existing pattern in `cmd_wait()` lines 1207-1262.
2. **Listen help mentions --since instead of --new-only**: A dedicated `--new-only` flag requires server-side changes because the `since` parameter in the SSE observe endpoint controls both history replay AND new message filtering. A client-only `--new-only` that sets `since=MAX` would suppress new messages. Instead, the help text now documents `--since <n>` as the mechanism to skip history replay, and `--new-only` is deferred to a future loop with server support.
3. **`tala use` listing**: In the "no args, no active session" branch, fetch sessions from daemon and print them. Reuse the same table format from `tala list` but keep it concise.
4. **Help text**: Add cross-reference lines to `#[command(after_help)]` for Wait, Stream, Listen. Mention `--new-session` in the Wait doc comment.
5. **Cursor in send**: Add `write_cursor(msg.id)` in `cmd_send()` after successful send. Uses the message ID from the `SendMessageResponse`.

## Risks / Trade-offs

- `--new-only` on listen is redundant if users always specify `--since`. But it's a common workflow shortcut for automated agents.
- Cursor update on send slightly changes behavior: after `tala send`, subsequent `tala list` won't show the user's own messages as new. This is the desired fix.
