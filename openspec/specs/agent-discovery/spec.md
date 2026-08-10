## Purpose

Agent discovery in tala: `tala agents` lists unique senders across open sessions, and `tala discover` finds tala projects in parent and sibling directories with their daemon status — so agents can find who is active, locally and across projects.

## Requirements

### Requirement: `tala agents` lists unique senders across open sessions

The system SHALL provide a `tala agents` command that lists all unique sender names across all open sessions, grouped by sender, showing last-seen timestamp and message count. Closed sessions SHALL be excluded.

#### Scenario: agents with messages

- **WHEN** user runs `tala agents` and messages exist in open sessions
- **THEN** the system SHALL display a table of unique senders with their sender name, last activity time, and message count

#### Scenario: agents with no messages

- **WHEN** user runs `tala agents` and there are no messages in any open session
- **THEN** the system SHALL display a message like "No active agents found. Start a session with `tala send`, or try `tala discover` to find agents in other projects."

#### Scenario: agents with closed sessions only

- **WHEN** user runs `tala agents` and all sessions are closed
- **THEN** the system SHALL display the "No active agents found" message

#### Scenario: agents --json

- **WHEN** user runs `tala agents --json`
- **THEN** the system SHALL output a JSON array with elements of shape `{"sender": "...", "last_seen": "<ISO-8601>", "message_count": N}`

### Requirement: Daemon endpoint for agents

The system SHALL provide a `GET /api/agents` endpoint that iterates open sessions and aggregates unique sender stats (last-seen timestamp and message count per sender).

#### Scenario: GET /api/agents

- **WHEN** daemon receives `GET /api/agents`
- **THEN** it SHALL return a JSON array of agent summaries derived from messages in open sessions
- **AND** each element SHALL carry `sender`, `last_seen` (ISO-8601), and `message_count`

### Requirement: `tala discover` finds tala projects in parent and sibling directories

The system SHALL provide a `tala discover` command that scans parent directories (up to 3 levels) and sibling directories under each common parent for `.tala/config.json` files, surfacing a list of known projects and their agent names. For each found config, the system SHALL read the agent name from the `name` field, attempt to connect to the daemon by reading `.tala/daemon.json` for host/port, and, if the daemon is reachable, query its `/api/agents` endpoint to list active agents.

#### Scenario: discover agents from parent projects

- **WHEN** user runs `tala discover`
- **THEN** the system SHALL scan parent directories (up to 3 levels) for `.tala/config.json` files
- **AND** for each found config, read the agent name from the `name` field
- **AND** attempt to connect to each daemon by reading `.tala/daemon.json` to get host/port
- **AND** if the daemon is reachable, query its `/api/agents` endpoint to list active agents
- **AND** display a table of discovered projects: project path, agent name, daemon status (running/stopped), active agents count

#### Scenario: discover agents from sibling projects

- **WHEN** user runs `tala discover` from `/workspace/project-a`
- **AND** `/workspace/project-b/.tala/config.json` exists
- **THEN** the system SHALL discover and list the agent from project-b

#### Scenario: no parent projects found

- **WHEN** user runs `tala discover` and no `.tala/config.json` is found in parent or sibling directories
- **THEN** the system SHALL display "No other tala projects discovered in parent directories."

#### Scenario: discover --json

- **WHEN** user runs `tala discover --json`
- **THEN** the output SHALL be a JSON array with elements `{"project": "...", "agent_name": "...", "daemon_running": bool, "agents": [{"sender": "...", "last_seen": "...", "message_count": N}]}`

### Requirement: Daemon status detection in `tala discover`

`tala discover` SHALL correctly detect running daemons in discovered projects. If a daemon's port responds to TCP connections but the `/api/agents` endpoint is unavailable, the daemon SHALL still be reported as "running".

#### Scenario: Discover running daemon via port probe

- **WHEN** a sibling project has a `.tala/daemon.json` file with a valid `host` and `port`
- **AND** the daemon's HTTP server is listening on that port
- **BUT** the `/api/agents` endpoint returns an error
- **THEN** `tala discover` SHALL report that project's daemon as "running"

#### Scenario: Discover stopped daemon

- **WHEN** a sibling project has no `.tala/daemon.json` file
- **OR** the daemon process is not actually running
- **THEN** `tala discover` SHALL report that project's daemon as "stopped"
