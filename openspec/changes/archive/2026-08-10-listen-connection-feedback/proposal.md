# Connection feedback for `listen` and `stream` (B007)

## Why

Backlog B007 (reconfirmed cycles 01–16): `tala listen` gives **zero** connected/
status feedback. `tala listen --timeout 4` with no new traffic produces empty
stdout AND stderr and exits 0; `--json` mode is identical. Even with live
traffic, text mode prints only message lines — no banner, no count, no
end-of-stream note.

For an agent whose job is to notice incoming messages (the receiver half of the
documented two-agent loop), "connected but quiet" and "command silently
no-op'd" are indistinguishable. `tala stream` shares the banner-less connect —
its only feedback is the end-state `[no messages received]` / `[session closed]`
line. This is the core "is the pipe alive" signal and it is missing entirely.

B007 is also B014-adjacent: `listen` defaults `--since` to the GLOBAL cursor
while message ids are per-session, so it can silently skip the newest message of
a low-id session (observed live this cycle: a message sent during the listen
window was never delivered). That root cause is tracked in B014/PR #46 — the
visibility fix here makes the *state of the connection* honest regardless.

## What Changes

CLI-side only (src/cli.rs). The daemon is untouched.

- **`cmd_listen`** (src/cli.rs): after a successful HTTP connect, print a
  connection banner; when the stream ends (server close / timeout), print a
  closed note carrying the count of messages received during the session.
- **`cmd_watch`** (stream, src/cli.rs): same connection banner after a
  successful connect. Existing end-state behavior (`[no messages received]` /
  `[session closed]`) is unchanged.
- **Channel discipline**: in `--json` mode the banner and closed note go to
  **stderr** so stdout stays a pure JSON event stream (machine-parseable); in
  text mode they go to **stdout** so a human watching the terminal sees them
  (matching the existing "Waiting for a new session (timeout: 45s)..." banner
  convention in `wait`).

## Capabilities

- `tala listen` (text): prints
  `Listening on tala daemon <host>:<port> (all sessions, since id <N>)...` on
  connect and `[connection closed] (<count> message(s))` on end.
- `tala listen --json`: stdout unchanged (JSON events only); stderr carries
  `[listen] connected to tala daemon <host>:<port> (since id <N>)` and
  `[listen] connection closed (<count> message(s))`.
- `tala stream <session>` (text): prints
  `Streaming session <id> from tala daemon <host>:<port> (since id <N>)...` on
  connect; `--json` mode puts the equivalent on stderr.

## Impact

- **Breaking change**: none — no stdout contract changes in `--json` mode; text
  mode gains lines on stdout that scripts piping text output should tolerate
  (message lines remain prefixed `[<label>]`, banners are distinct).
- **Tests**: new e2e coverage in tests/e2e.rs (banner presence, JSON stdout
  purity, closed-note with count); existing listen/stream e2e tests keep
  passing unchanged (all current listen tests run `--json` and assert on
  stdout only).
- **Docs**: no user-facing flag changes; `listen`/`stream` help text already
  describes behavior — optionally note the banner in after_help.
