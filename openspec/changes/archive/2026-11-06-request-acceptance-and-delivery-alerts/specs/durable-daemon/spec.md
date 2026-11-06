# Spec Delta

## MODIFIED Requirements

### Requirement: Protocol and lifecycle
The system SHALL reject incompatible wire versions before mutation, report status without starting a daemon, stop through the live service, and persist diagnostics. Blocking operations SHALL use an overall deadline and recover durable messages after restart. The daemon SHALL persist lifecycle and delivery events and evaluate notification deadlines while clients are blocked, including after restart.

#### Scenario: Due events are recovered after restart
- **WHEN** a request deadline or delivery threshold passes while the daemon is stopped
- **THEN** the daemon records the due event before returning from the next wait or pending operation

#### Scenario: Event writes and state transitions are atomic
- **WHEN** a decision or due notification is recorded
- **THEN** its state and event are committed together and cannot be duplicated by retry

#### Scenario: Restart recovery
- **WHEN** a daemon is killed after confirming a send
- **THEN** the replacement returns the same stored message and deduplication key

### Requirement: Thread-list protocol and storage versions match
The CLI and daemon SHALL use protocol version 10 for lifecycle coordination, thread listing, request decisions, and durable wait events. The daemon SHALL migrate SQLite schema version 5 to version 6 transactionally while preserving existing identities, messages, and receipts, and SHALL reject incompatible protocol versions before mutation.

#### Scenario: Existing database upgrades in place
- **WHEN** protocol 10 opens a schema version 5 database
- **THEN** it adds the required acceptance and event state transactionally without replacing identity or message rows

#### Scenario: Incompatible pre-list daemon
- **WHEN** a protocol 8 CLI contacts a protocol 9 daemon or vice versa
- **THEN** Tala reports protocol incompatibility before applying any operation
- **AND** the user is directed to stop an old daemon using its matching binary before upgrading

#### Scenario: Schema migration preserves existing data
- **WHEN** Tala opens a schema version 4 database
- **THEN** it creates and backfills thread-list summaries and access state in a transaction, preserves existing messages and receipts, and records schema version 5
