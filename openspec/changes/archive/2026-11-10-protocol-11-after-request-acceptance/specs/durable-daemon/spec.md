# Spec Delta

## MODIFIED Requirements

### Requirement: Thread-list protocol and storage versions match
The CLI and daemon SHALL use protocol version 11 for lifecycle coordination, thread listing, request decisions, durable wait events and network-window operations. The daemon SHALL migrate SQLite schema version 5 to version 6 transactionally while preserving existing identities, messages, and receipts, and SHALL reject incompatible protocol versions before mutation.

#### Scenario: Existing database upgrades in place
- **WHEN** protocol 11 opens a schema version 5 database
- **THEN** it adds the required acceptance and event state transactionally without replacing identity or message rows

#### Scenario: Incompatible pre-list daemon
- **WHEN** a protocol 10 CLI contacts a protocol 11 daemon or vice versa
- **THEN** Tala reports protocol incompatibility before applying any operation
- **AND** the user is directed to stop an old daemon using its matching binary before upgrading

#### Scenario: Schema migration preserves existing data
- **WHEN** Tala opens a schema version 4 database
- **THEN** it creates and backfills thread-list summaries and access state in a transaction, preserves existing messages and receipts, and records schema version 5

#### Scenario: Release 1.13 daemon
- **WHEN** a protocol 11 CLI finds a running protocol 10 daemon from v1.13.0
- **THEN** it reports `PROTOCOL_MISMATCH` before any operation and directs the user to stop that daemon with its matching binary
