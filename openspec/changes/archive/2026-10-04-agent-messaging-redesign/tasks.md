## 1. Contract
- [x] 1.1 Define complete replacement requirements and design; verify strict OpenSpec validation.
## 2. Core
- [x] 2.1 Implement typed protocol, agent registration and transactional SQLite store; verify identity, receipt, deduplication and corruption tests.
- [x] 2.2 Implement singleton daemon lifecycle and protocol checks; verify concurrent startup and restart tests.
- [x] 2.3 Implement complete CLI with explicit identity, inbox, reply, board, history, search, pending, resolve and handoff; verify end-to-end same-checkout and cross-project flows.
## 3. Integration
- [x] 3.1 Replace integration instructions, README and eval scenarios; verify documented commands and safe init behavior.
- [x] 3.2 Run build, tests, formatting and clippy; verify all required checks pass.
- [x] 3.3 Run a manually orchestrated real-agent scenario and record evidence; findings must be reviewed, no autonomous eval loop.
## 4. Completion
- [x] 4.1 Sync replacement specs, remove superseded empty capabilities and archive this change; verify openspec list is empty and validate --all passes.
