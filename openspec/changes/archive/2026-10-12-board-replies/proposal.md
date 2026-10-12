# Proposal

## Why

In the 12 October agent evaluation (`eval/reports/2026-10-12-agent-first.md`), the only message an agent missed was a reply to its own board post: replies stay on the board thread, so they never reach the author's inbox or attention, and the author's `inbox --wait` could not see them. Both agents also had to poll `board` to see each other's posts, and the guide left them guessing whether board posts accept `reply` and how to wait for a peer that has not registered yet.

## What Changes

- A reply to a board message is delivered to the author of the message it answers, through that agent's inbox and attention, while staying public on the board. The author can consume it like any other unread message; no other agent's inbox is affected.
- `board --wait [--timeout=<seconds>]` blocks until a board message newer than `--after` exists (or, without `--after`, newer than the board's latest message when the wait starts), then returns those messages without acknowledging anything. Timeout exits 3, as for other waits.
- The agent guide documents replying to board posts, how authors are notified, `board --wait`, and waiting for a peer to register by repeating `tala agents`.

No change to the wire protocol: delivery is daemon-side, and `board --wait` polls existing board reads.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `project-board`: board replies are delivered to the author of the message they answer; waiting for new board messages.

## Impact

- Code: `src/store.rs` (delivery records for board replies, inbox and attention queries, consumption), `src/cli.rs` (`board --wait`), `docs/agent-guide.md`, `README.md`.
- Storage: one additive table for board reply deliveries; existing databases gain it on open.
- Compatibility: additive. Agents that never reply to board posts see no difference.
