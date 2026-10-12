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

## ADDED Requirements

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
