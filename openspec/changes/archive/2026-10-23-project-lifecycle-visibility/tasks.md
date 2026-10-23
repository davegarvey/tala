# Tasks

## 1. Project storage and migration

- [x] 1.1 Add a transactional project-table migration that backfills registered projects with the earliest registered agent as owner and a seeded last-activity timestamp; verify migration tests preserve existing agents and messages.
- [x] 1.2 Create project records for new registrations and update last activity for registration and message creation; verify same-project agents share one owner and activity timestamp.

## 2. Lifecycle operations

- [x] 2.1 Add list, close and reopen project operations with owner authorization and status output; bump the wire protocol and verify owner/non-owner and closed-state tests.
- [x] 2.2 Derive the 30-day stale status and implement stale/closed inclusion filters and hidden counts; verify threshold boundaries and that activity clears stale without reopening closed projects.

## 3. CLI and documentation

- [x] 3.1 Add `list-projects`, `close-project` and `reopen-project` commands with JSON/human output, clear help and exact inclusion hints; verify CLI tests cover defaults, overrides and authorization errors.
- [x] 3.2 Document project lifecycle and the default listing behavior; verify the documented commands match `--help`.

## 4. Integration verification

- [x] 4.1 Run `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, `openspec validate project-lifecycle-visibility --strict`, and `git diff --check`; resolve all failures.

## Workflow follow-up

- Sync the delta specs into `openspec/specs/` and archive this change on the feature branch after implementation and validation.
