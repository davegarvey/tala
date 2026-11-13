# Tasks

## 1. Message validation

- [x] 1.1 Add boundary-aware detection for the generated private-handle format in text and structured data, with unit coverage for valid shapes, boundaries, malformed shapes, and public IDs.
- [x] 1.2 Reject matching content in the shared send path before idempotency or writes; add tests for send, reply, post, and no persistence or error disclosure.

## 2. User guidance

- [x] 2.1 Update registration output and identity guide to distinguish private credentials from public addresses; verify rendered text communicates the distinction.

## 3. Integration

- [x] 3.1 Run formatting, focused tests, full tests, and clippy with warnings denied; verify ordinary text and public agent IDs still work.

## Workflow follow-up

- Sync the delta specs into the main specs and archive this change on the feature branch.
