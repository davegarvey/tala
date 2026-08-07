# Restrict `--sender` to the configured agent identity (B004, decision)

## Why

`tala send --sender <any-name>` lets any local agent or process emit
messages as *any* identity, and `history`/`agents`/`discover` trust the
sender field verbatim. Backlog B004 (reconfirmed cycles 01–15). The interim
mitigation (PR #58) warned on mismatch but remained non-blocking — a spoofed
identity still reached recipients.

**Design decision (PO, 2026-08-07): Restrict.** `--sender` may only name the
project's configured agent (`.tala/config.json`); anything else is a hard
error. Rationale: tala's trust model is per-project identity — the honest way
to speak as another agent is to operate from that agent's project dir. The
"Authenticate" alternative (daemon-recorded unforgeable identity) is a larger
protocol change and can follow later if needed; restriction closes the hole
now with no protocol change.

## What Changes

- `cmd_send`: when `--sender` differs from the configured agent name, fail
  with exit 1 and a `SENDER_MISMATCH` error code (JSON-safe via the existing
  `fail()` contract) instead of warning and proceeding.
- Matching `--sender` (or none) behaves exactly as before.
- Remove the interim `sender_mismatch`/`configured_sender` fields from the
  success JSON ack — a mismatched sender can no longer reach a successful
  send, so the flag is dead weight.
- `send_content` drops its `configured_sender` parameter.

## Capabilities

- Agents can no longer impersonate other identities from a foreign project
  dir; `history`/`agents`/`discover` sender values become trustworthy.
- Scripts get a machine-readable `SENDER_MISMATCH` error in `--json` mode.

## Impact

- Behavior change (intentional): mismatched `--sender` previously warned and
  sent; now it errors and does not send.
- No daemon or wire-format change.
- e2e: rewrite the two B004-interim tests to assert the hard error; matching
  sender still succeeds.
