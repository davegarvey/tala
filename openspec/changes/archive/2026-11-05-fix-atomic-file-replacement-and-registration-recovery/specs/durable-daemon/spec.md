# Spec Delta

## MODIFIED Requirements

### Requirement: Exclusive durable service
The system SHALL operate one loopback daemon per Tala home, automatically start it safely under concurrent clients, and commit messages and receipts transactionally before success. Storage failure or corruption SHALL report an error without replacing storage with empty state.

#### Scenario: Concurrent startup
- **WHEN** multiple commands start against a fresh home
- **THEN** one daemon owns the home and all clients reach it

#### Scenario: Stale metadata restarted
- **WHEN** daemon metadata names a port on which connections are refused
- **THEN** the client starts a replacement daemon and the command succeeds

#### Scenario: Existing stale metadata replaced on Windows
- **WHEN** daemon metadata names a port on which connections are refused on Windows
- **THEN** the replacement daemon updates the existing metadata file and the command succeeds
