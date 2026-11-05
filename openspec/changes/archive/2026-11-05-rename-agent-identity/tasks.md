## Protocol and store
- [x] Add the authenticated rename RPC operation and bump protocol 4 to 5.
- [x] Preserve the authenticated identity in place, support same-name no-op and request-result idempotency, and test invalid credentials, blank names, duplicate request IDs and conflicting retries.

## CLI and docs
- [x] Add `tala rename --name=<new-name>` and human-readable output.
- [x] Update agent identity and CLI specs, then update the agent guide to distinguish rename from lost-handle recovery.
- [x] Add end-to-end coverage for preserved direct and board thread history, same ID/handle, duplicate-name ambiguity, same-name no-op and invalid names.

## Verify and archive
- [x] Run formatting, full tests, clippy and `openspec validate --all`.
- [x] Sync delta specs into main specs and archive this change on the feature branch.
