## Selector behavior
- [x] Add global `--agent-file` and `TALA_AGENT_FILE` support with explicit precedence and conflict validation.
- [x] Parse one bounded `th_` handle line with optional LF/CRLF and reject invalid, empty, oversized or extra content.
- [x] Add create-only `register --save-handle-to`; hide the handle on successful saves and disclose it once on save failure.
- [x] Reject non-UTF-8 save paths before registration so the selector path can be returned losslessly.

## File security
- [x] Implement Unix no-follow reads, current-user ownership/mode checks, and 0600 create-only writes without changing the parent.
- [x] Implement Windows protected current-user+SYSTEM DACL creation, owner/DACL validation, and reparse-point rejection.
- [x] Add Unix and Windows focused security coverage.

## Integration
- [x] Add end-to-end tests for selectors, precedence, malformed files, conflicts, saved output and save-failure recovery.
- [x] Update CLI help, README, agent guide and `agent-identity` spec.
- [x] Run formatting, tests, clippy and `openspec validate --all`.
- [x] Sync the delta spec and archive this change on the feature branch.
