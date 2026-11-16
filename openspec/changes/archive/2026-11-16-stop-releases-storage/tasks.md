# Tasks

## 1. Daemon shutdown order

- [x] 1.1 Hold the store in an `Option` and close it under the mutex before removing daemon metadata and releasing the lifetime lock; return `DAEMON_STOPPING` for requests after close.

## 2. Verification

- [x] 2.1 Run cargo fmt --check, cargo clippy --all-targets -- -D warnings, cargo test and openspec validate --all. The Windows CI job covers the deletion after stop in `status_reports_unavailable_local_context_without_creating_storage`.
