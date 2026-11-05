# Tasks

## 1. Durable message relationship

- [x] 1.1 Add the optional supersedes field to send operations and message output; keep absent-field serialization identical and verify existing canonical retry tests.
- [x] 1.2 Migrate SQLite schema version 2 to 3 with a nullable self-reference and unique successor index; add migration coverage and verify existing rows remain readable.
- [x] 1.3 Bump protocol version to 6 and validate matching/mismatching daemon behavior with the protocol tests.
- [x] 1.4 Validate target ownership, recipient, kind, thread inference, earlier-ID constraint, and linear chain in the send transaction; add store tests for valid chains and each rejected target class.

## 2. Request obligations and replies

- [x] 2.1 Expose `supersedes` and derived `superseded_by` in inbox/history/message results and verify both directions survive store reopen without changing receipt or answer timestamps.
- [x] 2.2 Exclude superseded requests from pending, stale counts, and attention; add store tests for incoming and outgoing requests while confirming unread rows remain unread.
- [x] 2.3 Reject replies to superseded requests with `REQUEST_SUPERSEDED`, successor ID, and exact thread-inspection guidance; add a store and CLI test proving no reply is stored.

## 3. CLI surface and documentation

- [x] 3.1 Add `send --supersedes=<id>` and conditional auto-key material; test same-target retry deduplication, changed-target explicit-key conflict, and unchanged ordinary derived keys.
- [x] 3.2 Render amendment relationships in human output and document send, validation, chain, pending, and stale-reply behavior in help and the agent guide; verify rendered output.
- [x] 3.3 Sync the OpenSpec delta to main specs, archive the change, and verify `openspec validate --all` succeeds with no active changes.

## 4. Integration verification

- [x] 4.1 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, full `cargo test`, and `git diff --check`; resolve all failures before opening the PR.
