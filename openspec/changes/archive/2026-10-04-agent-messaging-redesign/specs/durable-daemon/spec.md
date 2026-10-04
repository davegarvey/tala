## Purpose

Defines durable daemon for reliable local collaboration between distinct coding agents in shared and separate projects.

## ADDED Requirements

### Requirement: Exclusive durable service
The system SHALL operate one loopback daemon per Tala home, automatically start it safely under concurrent clients, and commit messages and receipts transactionally before success. Storage failure or corruption SHALL report an error without replacing storage with empty state.

#### Scenario: Concurrent startup
- **WHEN** multiple commands start against a fresh home
- **THEN** one daemon owns the home and all clients reach it

### Requirement: Protocol and lifecycle
The system SHALL reject incompatible wire versions before mutation, report status without starting a daemon, stop through the live service, and persist diagnostics. Blocking operations SHALL use an overall deadline and recover durable messages after restart.

#### Scenario: Restart recovery
- **WHEN** a daemon is killed after confirming a send
- **THEN** the replacement returns the same stored message and deduplication key

### Requirement: Legacy files preserved
The system SHALL leave old JSON state and project identity/cursors untouched, use fresh new storage, report legacy state in status and document explicit upgrade/rollback.

#### Scenario: Old state
- **WHEN** a home contains legacy messages.json
- **THEN** new registration leaves that file unchanged and status reports its presence
