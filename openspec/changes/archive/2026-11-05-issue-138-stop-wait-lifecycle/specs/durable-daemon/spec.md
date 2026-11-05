## ADDED Requirements

### Requirement: Stop and startup are linearizable
Daemon startup SHALL reserve the current lifecycle generation before attempting startup coordination and SHALL publish an instance only while that generation remains current. An accepted stop SHALL advance the generation, prevent earlier reservations from publishing afterward, target the instance active at stop acceptance, and report success only after that instance exits. No lifecycle or startup lock SHALL be held while waiting for shutdown.

#### Scenario: Stop races with an unpublished start
- **WHEN** a start reservation exists but has not published its daemon instance when stop is accepted
- **THEN** that reservation cannot publish after the stop generation advances
- **AND** stop-in-progress remains set even when no daemon target has published
- **AND** stop waits for the startup barrier before reporting success

#### Scenario: Stop targets the active instance
- **WHEN** stop is accepted while an instance is active and a later explicit command starts a replacement
- **THEN** stop waits for the originally targeted instance to exit
- **AND** the replacement is not mistaken for the target or killed by that stop

#### Scenario: Explicit command starts after stop
- **WHEN** an ordinary explicit command needs a daemon while an instance is stopping
- **THEN** it waits without holding lifecycle locks and may start a replacement after the target exits

#### Scenario: Stop marker survives an in-flight stop
- **WHEN** the target removes daemon metadata before releasing its lifetime lock
- **THEN** stop-in-progress remains set until that lock is available
- **AND** a new wait exits terminally while that state remains set

### Requirement: Stop generations preserve crash recovery
The CLI SHALL distinguish accepted graceful stops from ungraceful daemon failures using a monotonically increasing per-home stop generation. A crash SHALL NOT advance the generation. A waiter SHALL recover through the existing startup coordination when its captured generation remains current, and SHALL NOT recover after the generation changes.

#### Scenario: Wait recovers after a crash
- **WHEN** a daemon crashes while a wait is active without an accepted stop
- **THEN** the generation remains unchanged and the waiter can use the existing startup recovery path

#### Scenario: Wait observes intentional stop
- **WHEN** stop advances the generation while a wait is active
- **THEN** the waiter does not start or adopt a replacement daemon

## MODIFIED Requirements

### Requirement: Thread-list protocol and storage versions match
The CLI and daemon SHALL use protocol version 9 for lifecycle coordination and thread listing, and the daemon SHALL migrate SQLite schema version 4 to version 5 transactionally before serving the new operation. The daemon SHALL reject incompatible versions before mutation.

#### Scenario: Incompatible pre-list daemon
- **WHEN** a protocol 9 client contacts a protocol 8 daemon or vice versa
- **THEN** Tala reports `PROTOCOL_MISMATCH` before applying the requested operation
- **AND** the user is directed to stop an old daemon using its matching binary before upgrading

#### Scenario: Schema migration preserves existing data
- **WHEN** Tala opens a schema version 4 database
- **THEN** it creates and backfills thread-list summaries and access state in a transaction, preserves existing messages and receipts, and records schema version 5
