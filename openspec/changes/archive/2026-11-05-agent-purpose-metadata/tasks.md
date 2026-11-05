# Tasks

## 1. Purpose validation and storage

- [x] 1.1 Add purpose validation and normalization; test absent, trimming, scalar-count max/+1, multibyte Unicode, control characters, bidi overrides, and isolates.
- [x] 1.2 Add nullable purpose fields and bump protocol to 7; add the transactional schema v3→v4 migration and test v3 plus fresh database initialization preserves identity/message rows.
- [x] 1.3 Extend registration and agent projection with purpose fields; test initial purpose timestamp, null legacy values, and project-filtered `agents` parity.

## 2. Purpose update operation and CLI

- [x] 2.1 Add identity-bound `update-purpose --purpose` / `--clear`, cached mutation behavior, and no presence refresh; test set/clear retry returns a stable result and does not change `last_seen`, identity, messages, or history.
- [x] 2.2 Render purpose in `whoami --human` and `agents --human`; test readable output, defensive escaping, 80-scalar truncation, and full JSON values.
- [x] 2.3 Document peer visibility and untrusted-data limits in CLI help and update agent-identity/agent-cli OpenSpecs.

## 3. Integration and delivery checks

- [x] 3.1 Run formatting, Clippy, complete tests, build, and OpenSpec validation; confirm protocol mismatch rejects an old client before mutation.

## Workflow follow-up

- [x] Sync the delta specs into the main specs and archive this change on the feature branch before opening the PR.
