# Tasks

## 1. Request completion operation

- [x] 1.1 Add the CLI command and wire operation; verify help documents the required answer ID and CLI parsing tests cover it.
- [x] 1.2 Implement transactional authorization, answer validation, idempotence, durable event, and open-request removal; verify unit tests cover valid, invalid, and conflicting cases.

## 2. Actionable attention and validation

- [x] 2.1 Exclude declined requests from pending and attention output while retaining history; verify store tests cover pending and thread summaries.
- [x] 2.2 Update user guidance and relevant CLI/spec text; verify fmt, clippy, and targeted isolated-home CLI tests pass.

## Workflow follow-up

- Sync delta specs into main specs and archive this change after implementation.
- Coordinate protocol version 11 to 12 on the integration branch.
