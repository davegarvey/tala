## ADDED Requirements

### Requirement: Daemon status detection
`tala discover` SHALL correctly detect running daemons in discovered sibling projects. If a daemon's port responds to TCP connections but the `/api/agents` endpoint is unavailable, the daemon SHALL still be reported as "running".

#### Scenario: Discover running daemon via port probe
- **WHEN** a sibling project has a `.tala/daemon.json` file with a valid `host` and `port`
- **AND** the daemon's HTTP server is listening on that port
- **BUT** the `/api/agents` endpoint returns an error
- **THEN** `tala discover` SHALL report that project's daemon as "running"

#### Scenario: Discover stopped daemon
- **WHEN** a sibling project has no `.tala/daemon.json` file
- **OR** the daemon process is not actually running
- **THEN** `tala discover` SHALL report that project's daemon as "stopped"

### Requirement: Agent visibility in agents command
`tala agents` SHALL show agents from all discovered projects with running daemons, even if no messages have been sent or received yet.

#### Scenario: Agents visible before messaging
- **WHEN** `tala agents` is run
- **AND** there are sibling projects with running daemons
- **AND** sessions exist but no messages have been exchanged
- **THEN** the agents from those sibling projects SHALL appear in the output

#### Scenario: Agents visible from message history
- **WHEN** `tala agents` is run
- **AND** messages have been exchanged in active sessions
- **THEN** all unique senders from those messages SHALL appear in the output
