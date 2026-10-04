## Why

Project identity and shared session cursors cannot distinguish agents in one checkout. Agents need a complete CLI for addressed conversations and shared updates, without consulting internal logs.

## What Changes

- **BREAKING** Replace implicit sessions and project identities with explicit registered agent instances, direct inboxes, automatic reply routing, and project boards.
- **BREAKING** Replace JSON transcript persistence and legacy wire shapes with a versioned SQLite-backed daemon protocol; retain old files untouched without automatic import.
- Provide registration, discovery, sending, receiving, replies, posting, history, search, pending work, handoffs, and delivery inspection entirely through the CLI.
- Provide bounded JSON output, stable global IDs, incremental reads, explicit receipts, reliable retries, and exclusive daemon ownership.
- Replace integration instructions and evaluation scenarios around the new contract.

## Capabilities

### New Capabilities
- `agent-identity`: Independent agent registration, selection, presence, and discovery.
- `agent-messaging`: Addressed messages, replies, obligations, delivery, and handoffs.
- `project-board`: Shared updates, threads, search, and non-consuming history.
- `agent-cli`: Complete command contract, input sources, output, errors, and integrations.
- `durable-daemon`: Transactional persistence, singleton lifecycle, protocol checks, and legacy-state handling.

### Modified Capabilities
- `wait-all`: Retire the complete session-oriented contract; superseded by the five capabilities above.
- `stdin-sending`: Retire the complete session-oriented contract; superseded by the five capabilities above.
- `skill-cli-compatibility`: Retire the complete session-oriented contract; superseded by the five capabilities above.
- `agent-discovery`: Retire the complete session-oriented contract; superseded by the five capabilities above.
- `message-observation`: Retire the complete session-oriented contract; superseded by the five capabilities above.
- `cli`: Retire the complete session-oriented contract; superseded by the five capabilities above.
- `session-lifecycle`: Retire the complete session-oriented contract; superseded by the five capabilities above.
- `send-idempotency`: Retire the complete session-oriented contract; superseded by the five capabilities above.
- `project-setup`: Retire the complete session-oriented contract; superseded by the five capabilities above.
- `sessions`: Retire the complete session-oriented contract; superseded by the five capabilities above.
- `message-read-state`: Retire the complete session-oriented contract; superseded by the five capabilities above.
- `daemon`: Retire the complete session-oriented contract; superseded by the five capabilities above.
- `message-intent`: Retire the complete session-oriented contract; superseded by the five capabilities above.
- `message-parts`: Retire the complete session-oriented contract; superseded by the five capabilities above.
- `daemon-compat`: Retire the complete session-oriented contract; superseded by the five capabilities above.
- `message-waiting`: Retire the complete session-oriented contract; superseded by the five capabilities above.
- `message-sending`: Retire the complete session-oriented contract; superseded by the five capabilities above.
- `waiting-coordination`: Retire the complete session-oriented contract; superseded by the five capabilities above.

## Impact

Replace src/{cli,models,store,api,daemon}.rs and session-focused tests. Add bundled SQLite and file-lock dependencies. Update README, embedded integrations, eval scenarios, and main specs. Installation remains unchanged. No network federation, autonomous scheduling, file locks for coding work, or automatic execution of peer instructions.
