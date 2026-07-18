## ADDED Requirements

### Requirement: tala discover SHALL discover agents in parent projects
The system SHALL provide a `tala discover` command that scans parent directories for `.tala/config.json` files and running daemons, surfacing a list of known projects and their agent names.

#### Scenario: discover agents from parent projects
- **WHEN** user runs `tala discover`
- **THEN** the system SHALL scan parent directories (up to 3 levels) for `.tala/config.json` files
- **AND** for each found config, read the agent name from the `name` field
- **AND** attempt to connect to each daemon by reading `.tala/daemon.json` to get host/port
- **AND** if the daemon is reachable, query its `/api/agents` endpoint to list active agents
- **AND** display a table of discovered projects: project path, agent name, daemon status (running/stopped), active agents count

#### Scenario: discover agents --json
- **WHEN** user runs `tala discover --json`
- **THEN** the output SHALL be a JSON array with elements: `{"project": "...", "agent_name": "...", "daemon_running": bool, "agents": [{"sender": "...", "last_seen": "...", "message_count": N}]}`

#### Scenario: no parent projects found
- **WHEN** user runs `tala discover` and no `.tala/config.json` is found in parent directories
- **THEN** the system SHALL display "No other tala projects discovered in parent directories."

### Requirement: tala discover SHALL check siblings in common parent workspaces
When scanning for projects, the system SHALL also look for `.tala/config.json` in sibling directories under a common parent (e.g. if running from `/projects/foo`, check `/projects/bar/.tala/config.json`).

#### Scenario: discover agents from sibling projects
- **WHEN** user runs `tala discover` from `/workspace/project-a`
- **AND** `/workspace/project-b/.tala/config.json` exists
- **THEN** the system SHALL discover and list the agent from project-b
