# Proposal

## Why

Long-running agent work can continue against an instruction after its sender has corrected or replaced it. Tala stores both messages in order but does not identify which instruction is current, so recipients can miss an amendment and waste work or produce invalid results.

## What Changes

- Add an explicit `--supersedes=<message-id>` relationship for direct messages.
- Preserve the original and amended messages and expose both directions of the relationship.
- Stop counting superseded requests as outstanding, and reject replies to a superseded request with the successor's ID.
- Validate that amendments form a linear chain within one sender, recipient, and direct thread.
- Bump the CLI-daemon protocol and migrate storage for the new relationship.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `agent-messaging`: direct messages may supersede earlier messages; superseded requests cease to be reply obligations while remaining in history.
- `agent-attention`: incoming and outgoing attention excludes superseded requests.
- `agent-cli`: send accepts `--supersedes`, renders amendment relationships, and returns an actionable stale-reply error.

## Impact

This affects the send CLI and wire operation, message storage and schema migration, idempotency canonicalization, message serialization, inbox/history/pending/attention output, reply validation, and related tests. Existing messages remain unchanged; the relationship is nullable and additive. Protocol version 6 prevents older daemons from silently ignoring amendment metadata.
