# Tasks

- [x] 1. Add acceptance state, optional deadline and durable notification tables with additive migration and indexes.
- [x] 2. Implement idempotent accept/decline, reply completion, cancellation/supersession races and deadline expiry events.
- [x] 3. Add configurable unread delivery-overdue scheduling and receiver listening hints; keep inactive status informational.
- [x] 4. Extend pending, inbox, CLI commands and guidance for acceptance and overdue states.
- [x] 5. Replace client-side repeated wait reads with one daemon-held, cursor-aware wait that covers inbox, board and durable events.
- [x] 6. Add store, CLI and integration coverage for races, idempotency, restart, deadlines, overdue delivery and the long-blocking wait contract.
- [x] 7. Update README, agent guide, main specs and protocol/schema docs; run formatting, lint, unit, e2e and change-scoped strict OpenSpec validation (full-repository strict mode still reports existing long requirement descriptions).
- [x] 8. Review the final diff, open and review a PR, merge only with passing checks, then safely update and verify the local Tala binary.
