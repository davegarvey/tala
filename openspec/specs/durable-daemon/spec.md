## Purpose

Defines durable daemon for reliable local collaboration between distinct coding agents in shared and separate projects.

## Requirements

### Requirement: Exclusive durable service
The system SHALL operate one loopback daemon per Tala home, automatically start it safely under concurrent clients, and commit messages and receipts transactionally before success. Storage failure or corruption SHALL report an error without replacing storage with empty state. Clients SHALL acquire startup coordination without blocking their async executor, bound waits for another starter, and release the startup lock as soon as they confirm a healthy daemon before sending long-lived requests.

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

### Requirement: Protocol and lifecycle
The system SHALL reject incompatible wire versions before mutation, report status without starting a daemon, stop through the live service, and persist diagnostics. Blocking operations SHALL use an overall deadline and recover durable messages after restart.

#### Scenario: Restart recovery
- **WHEN** a daemon is killed after confirming a send
- **THEN** the replacement returns the same stored message and deduplication key

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

### Requirement: Legacy files preserved
The system SHALL leave old JSON state and project identity/cursors untouched, use fresh new storage, report legacy state in status and document explicit upgrade/rollback.

#### Scenario: Old state
- **WHEN** a home contains legacy messages.json
- **THEN** new registration leaves that file unchanged and status reports its presence

### Requirement: Unreachable daemon is not presumed dead
When connecting to the recorded daemon fails for any reason other than refusal, including permission denial or timeout, the client SHALL not start another daemon and SHALL fail with `DAEMON_UNREACHABLE`, preserving the underlying error and a hint that sandbox or network restrictions may block loopback access.

#### Scenario: Sandbox blocks loopback
- **WHEN** a command runs where loopback connections are denied while a daemon is live
- **THEN** it fails with `DAEMON_UNREACHABLE`, no second daemon is spawned, and the error retains the operating system's reason

### Requirement: Transport failures keep their meaning
Transport failures SHALL be reported only as transport error codes and SHALL never be mapped to application codes such as recipient or message not found.

#### Scenario: Connection lost during lookup
- **WHEN** the connection fails while resolving a recipient
- **THEN** the error code is a transport code, not `RECIPIENT_NOT_FOUND`

### Requirement: Status context inspection never starts or mutates the daemon
Status SHALL read selected identity context from a read-only local database snapshot and SHALL not start the daemon or refresh agent presence.

#### Scenario: Daemon stopped or stale metadata
- **WHEN** status runs while the daemon is stopped, including when stale daemon metadata remains, and the local database is readable
- **THEN** it returns daemon diagnostics and selector source without starting a daemon
- **AND** it reports identity, project root, and board latest ID from one local database snapshot

#### Scenario: Daemon incompatible or unreachable
- **WHEN** status cannot use the recorded daemon because its protocol is incompatible or the daemon is unreachable, and the local database is readable
- **THEN** it preserves the existing compatibility/reachability diagnostics and reports identity context from the local database snapshot
- **AND** it does not start another daemon

#### Scenario: Status does not refresh presence or receipts
- **WHEN** status reports context for a valid selected identity
- **THEN** its read-only snapshot does not change the identity's last-seen or active fields
- **AND** it does not consume inbox or board deliveries or mutate receipts

#### Scenario: Local database cannot be read
- **WHEN** the local database is missing, locked beyond the bounded read timeout, or unreadable
- **THEN** status preserves existing daemon diagnostics and reports local identity context as unavailable
- **AND** it does not create or replace the database

### Requirement: Thread listing uses immutable identity-bound snapshots
The daemon SHALL capture the thread-list high-water message ID and all response rows, including visibility and unread/pending values, in one transaction. Later pages SHALL return those materialized rows unchanged by activity or receipt updates after capture.

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
Thread-list cursors SHALL be bound to the authenticated identity. Snapshots SHALL expire after 15 minutes and at most eight unexpired snapshots SHALL be retained per agent. Invalid cursors SHALL return the same generic `INVALID_CURSOR` failure.

#### Scenario: Expired or evicted cursor
- **WHEN** an agent continues a list with an expired or evicted cursor
- **THEN** Tala returns the generic `INVALID_CURSOR` error and directs the agent to start a fresh listing

#### Scenario: Cursor belongs to another identity
- **WHEN** an agent presents a cursor created for a different identity
- **THEN** Tala returns the same generic `INVALID_CURSOR` error without revealing cursor ownership or state

#### Scenario: Snapshot retention is bounded
- **WHEN** an agent creates a ninth unexpired snapshot
- **THEN** Tala evicts that agent's oldest snapshot and its cursors

### Requirement: Thread-list protocol and storage versions match
The CLI and daemon SHALL use protocol version 9 for lifecycle coordination and thread listing, and the daemon SHALL migrate SQLite schema version 4 to version 5 transactionally before serving the new operation. The daemon SHALL reject incompatible versions before mutation.

#### Scenario: Incompatible pre-list daemon
- **WHEN** a protocol 8 CLI contacts a protocol 9 daemon or vice versa
- **THEN** Tala reports protocol incompatibility before applying any operation
- **AND** the user is directed to stop an old daemon using its matching binary before upgrading

#### Scenario: Schema migration preserves existing data
- **WHEN** Tala opens a schema version 4 database
- **THEN** it creates and backfills thread-list summaries and access state in a transaction, preserves existing messages and receipts, and records schema version 5
