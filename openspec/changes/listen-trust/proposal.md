# Make listen trustworthy as the monitor command (B046)

## Why

B046 (cycle-19 broadcast eval, monitor agent): `tala listen` — the keep-command
for real-time monitoring — is unreliable:

1. **`--timeout 0` is not honored**: cli.rs `timeout.filter(|&t| t != 0).or(Some(60))`
   turns `--timeout 0` into 60s, so connections die after a minute despite the
   documented "0 = no timeout" (explains "2 of 3 connections died ~3 min in").
2. **No cursor advancement**: listen never writes the per-session cursors it
   delivers, so `check` re-shows delivered messages and a reconnect replays
   them (bc-β: "listen doesn't advance the shared cursor").
3. **Silent drops on broadcast lag**: `Err(RecvError::Lagged) => continue`
   skips the lagged messages with no signal — a burst can lose messages
   without the monitor noticing (the missed-message symptom class).

## What Changes

- **cli.rs cmd_listen**: honor `--timeout 0` (indefinite); default stays 60.
- **cli.rs cmd_listen**: write the per-session cursor for each delivered
  message (store::write_cursor), so delivered messages are seen-as-read by
  check and not replayed on reconnect. Explicit `--since`/`--since_map`
  behavior unchanged (replay mode).
- **api.rs observe_events**: on `RecvError::Lagged(n)`, emit a typed
  `overload` SSE event (count skipped) instead of silently continuing; the
  CLI renders it as a stderr warning. No message is fabricated; the monitor
  learns it must run `tala check`.

## Capabilities

- `listen --timeout 0` stays connected indefinitely.
- `listen` is a replay-free monitor: delivered messages don't reappear in
  `check` or on reconnect.
- A lagged/bursty daemon warns instead of dropping silently.

## Impact

- CLI + daemon; additive SSE event type (old CLIs ignore unknown event
  types — `_ => {}` in the parser). No wire/protocol bump.
- e2e: new tests (timeout-0 stays connected and delivers; delivered messages
  don't replay; check agrees after listen).
- Evidence: /workspace/tala-po/feedback/cycle-19/broadcast-beta.md.
