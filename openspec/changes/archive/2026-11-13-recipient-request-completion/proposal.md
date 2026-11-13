# Proposal

## Why

Recipients can send a real answer to a request but cannot close the request unless they reply in that request's thread. Declined requests also continue to appear as actionable outgoing pending work, obscuring what still needs attention.

## What Changes

- Add a recipient-side completion command that links an already-sent answer to its request.
- Validate that the linked message is a newer, ordinary direct message from the recipient to the request sender.
- Record completion as an auditable request event while retaining the original answer message and its thread.
- Exclude declined requests from actionable pending and attention counts while preserving their decision and history.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-messaging`: allow a recipient to link a valid existing answer and close the request.
- `agent-cli`: expose the completion action and keep declined requests out of actionable pending lists.
- `agent-attention`: ensure declined and completed requests no longer count as actionable attention.

## Impact

The CLI gains `tala complete <request-id> --answered-by=<message-id>`. The daemon wire operation is additive but is not understood by old daemons; the integration branch must bump `PROTOCOL_VERSION` from 11 to 12. SQLite reuses the existing `answered_by` relation and event table; no schema migration is needed.
