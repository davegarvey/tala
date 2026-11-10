# Spec Delta

## MODIFIED Requirements

### Requirement: Status context inspection never starts or mutates the daemon
Status SHALL read selected identity context from a read-only local database snapshot and SHALL not start the daemon or refresh agent presence. In a home connected to a remote daemon, status SHALL report identity context as unavailable rather than reading a local database.

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

#### Scenario: Home connected to a remote daemon
- **WHEN** status runs in a home connected to a remote daemon
- **THEN** it queries the remote daemon's status, reports identity context as unavailable with reason `remote_daemon`, and starts no local daemon
