# Spec Delta

## MODIFIED Requirements

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

#### Scenario: Legacy startup lock held after upgrade
- **WHEN** a process from an older release still holds the legacy startup lock
- **THEN** a current client can start or reach the daemon without waiting for that legacy lock
- **AND** concurrent current clients still use one daemon per Tala home
