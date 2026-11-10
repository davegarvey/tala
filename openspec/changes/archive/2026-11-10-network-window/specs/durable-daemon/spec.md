# Spec Delta

## MODIFIED Requirements

### Requirement: Exclusive durable service
The system SHALL operate one daemon per Tala home that always listens on loopback and, only while a network window is open for that home, also listens on the window's network address. It SHALL automatically start that daemon safely under concurrent clients, and commit messages and receipts transactionally before success. A home connected to a remote daemon SHALL NOT start a local daemon. Storage failure or corruption SHALL report an error without replacing storage with empty state. Clients SHALL acquire startup coordination without blocking their async executor, bound waits for another starter, and release the startup lock as soon as they confirm a healthy daemon before sending long-lived requests.

#### Scenario: Concurrent startup
- **WHEN** multiple commands start against a fresh home
- **THEN** one daemon owns the home and all clients reach it

#### Scenario: Stale metadata restarted
- **WHEN** daemon metadata names a port on which connections are refused
- **THEN** the client starts a replacement daemon and the command succeeds

#### Scenario: Existing stale metadata replaced on Windows
- **WHEN** daemon metadata names a port on which connections are refused on Windows
- **THEN** the replacement daemon updates the existing metadata file and the command succeeds

#### Scenario: Wait recovers when the daemon dies
- **WHEN** a daemon dies while a long-lived `wait` has concurrent inbox and board requests, and another command needs the daemon
- **THEN** startup-lock contention yields while one client recovers the daemon, the other command uses the recovered daemon or returns a bounded actionable error, and no client requires manual process cleanup

#### Scenario: Legacy startup lock held after upgrade
- **WHEN** a process from an older release still holds the legacy startup lock
- **THEN** a current client can start or reach the daemon without waiting for that legacy lock
- **AND** concurrent current clients still use one daemon per Tala home

### Requirement: Thread-list protocol and storage versions match
The CLI and daemon SHALL use protocol version 10 for lifecycle coordination, thread listing and network-window operations, and the daemon SHALL migrate SQLite schema version 4 to version 5 transactionally before serving the new operation. The daemon SHALL reject incompatible versions before mutation.

#### Scenario: Incompatible pre-list daemon
- **WHEN** a protocol 9 CLI contacts a protocol 10 daemon or vice versa
- **THEN** Tala reports protocol incompatibility before applying any operation
- **AND** the user is directed to stop an old daemon using its matching binary before upgrading

#### Scenario: Schema migration preserves existing data
- **WHEN** Tala opens a schema version 4 database
- **THEN** it creates and backfills thread-list summaries and access state in a transaction, preserves existing messages and receipts, and records schema version 5
