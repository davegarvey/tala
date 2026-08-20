## Why

The current intent protocol correctly tracks whether requests are answered, but
the human CLI can present contradictory signals during a live exchange. After a
correlated reply, `history` can still show the original request as actively
waiting, `send --wait` does not identify the received message, and `pending`
suggests a reply command without a session target. These small gaps make
multi-session agent conversations harder to complete safely.

## What Changes

- Reconcile wait-deadline rendering with reply correlation: an answered request
  SHALL no longer look like an active wait in transcript output.
- Include the received message ID and reply correlation in human-readable
  `send --wait` output, while preserving structured JSON output.
- Include an explicit session target in the reply command suggested by
  `pending`.
- Keep the intent model, pending semantics, and machine-readable stdout
  contract unchanged. The settled-rendering behavior applies to the remaining
  public message surfaces while the separate command-surface change removes
  `stream`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `message-intent`: Clarify how answered requests render after their stamped
  wait deadline and how pending reply guidance targets a session.
- `cli`: Make blocking-send reply output identify the received message and its
  correlation target.

## Impact

- Human-readable rendering in `src/cli.rs` and any supporting message/state
  model logic.
- CLI/e2e tests covering intent correlation, `send --wait`, `history`, and
  `pending` output.
- No daemon wire-protocol, persistence, or new command/flag changes are
  expected.
