# Spec Delta

## ADDED Requirements

### Requirement: Thread listing uses immutable identity-bound snapshots
The daemon SHALL capture the thread-list high-water message ID and all response rows, including visibility and unread/pending values, in one transaction. Later pages SHALL read those materialized rows unchanged by activity or receipt updates after capture.

#### Scenario: Existing thread changes between pages
- **WHEN** a thread receives a newer message after page one is returned
- **THEN** later pages preserve the original row and order, while a fresh list includes the new activity

#### Scenario: Receipt or request state changes between pages
- **WHEN** a delivery is acknowledged or a request is answered, cancelled, or superseded after page one
- **THEN** later pages preserve the original unread and pending values

#### Scenario: Visibility changes between pages
- **WHEN** participation or a direct handoff grant changes after page one
- **THEN** later pages preserve the original thread membership and access-recipient roles

### Requirement: Thread-list cursors are bounded and non-disclosing
Thread-list cursors SHALL be versioned and bound to the authenticated public agent ID. Snapshots SHALL expire after 15 minutes and at most eight unexpired snapshots SHALL be retained per agent. Malformed, expired, evicted, or other-agent cursors SHALL return the same generic `INVALID_CURSOR` failure.

#### Scenario: Expired or evicted cursor
- **WHEN** an agent continues a list with an expired or evicted cursor
- **THEN** Tala returns the generic `INVALID_CURSOR` error and directs the agent to start a fresh listing

#### Scenario: Cursor belongs to another identity
- **WHEN** an agent presents a cursor created for a different identity
- **THEN** Tala returns the same generic `INVALID_CURSOR` error without revealing cursor ownership or state

#### Scenario: Snapshot retention is bounded
- **WHEN** an agent creates a ninth unexpired snapshot
- **THEN** Tala evicts that agent's oldest snapshot and its cursors

### Requirement: Thread listing protocol and storage migrate together
The CLI and daemon SHALL use protocol version 8 for thread listing, and the daemon SHALL migrate SQLite schema version 4 to version 5 transactionally before serving the new operation.

#### Scenario: Incompatible pre-list daemon
- **WHEN** a protocol 7 CLI contacts a protocol 8 daemon or vice versa
- **THEN** Tala reports protocol incompatibility before applying any operation

#### Scenario: Schema migration preserves existing data
- **WHEN** Tala opens a schema version 4 database
- **THEN** it creates and backfills thread-list summaries and access state in a transaction, preserves existing messages and receipts, and records schema version 5
