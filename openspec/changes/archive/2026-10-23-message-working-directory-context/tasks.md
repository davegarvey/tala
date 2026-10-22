# Tasks

## 1. Message context

- [x] 1.1 Add an optional working-directory field to context and populate it from the current directory on outgoing commands; verify registered project, worktree and board identity remain unchanged.
- [x] 1.2 Add e2e coverage for changing directories after registration and verify outgoing message context reports both registered project and current working directory.
- [x] 1.3 Update agent guide context descriptions and verify examples match emitted JSON.

## 2. Validation

- [x] 2.1 Run focused context and messaging tests, then `cargo fmt --check`, `cargo test`, Clippy with warnings denied, and `openspec validate --all`.
