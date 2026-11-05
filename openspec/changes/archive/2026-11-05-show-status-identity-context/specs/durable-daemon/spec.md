# Spec Delta

## ADDED Requirements

### Requirement: Status context inspection never starts the daemon
Status SHALL inspect selected identity context only through a currently reachable compatible daemon and SHALL leave identity, inbox, board, and daemon state unchanged.

#### Scenario: Daemon stopped or stale metadata
- **WHEN** status runs while the daemon is stopped, including when stale daemon metadata remains
- **THEN** it returns daemon diagnostics and selector source without starting a daemon
- **AND** it marks daemon-dependent identity name, project root, and board latest ID unavailable with a reason

#### Scenario: Daemon incompatible or unreachable
- **WHEN** status cannot use the recorded daemon because its protocol is incompatible or the daemon is unreachable
- **THEN** it preserves the existing compatibility/reachability diagnostics and reports daemon-dependent identity context unavailable
- **AND** it does not start another daemon

#### Scenario: Live daemon context inspection
- **WHEN** status has a valid selected identity and a compatible reachable daemon
- **THEN** it reads that identity's public details and registered project board high-water mark
- **AND** it does not consume inbox or board deliveries or mutate receipts
