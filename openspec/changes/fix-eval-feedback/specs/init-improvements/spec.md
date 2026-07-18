## ADDED Requirements

### Requirement: init --json flag
`tala init` SHALL accept a `--json` / `-j` flag that outputs structured JSON instead of human-readable text.

#### Scenario: init --json on first run
- **WHEN** user runs `tala init --json` in a project with no `.tala/config.json`
- **THEN** output SHALL be valid JSON: `{"name": "<project-name>", "daemon_started": true}`

#### Scenario: init --json when already initialized
- **WHEN** user runs `tala init --json` in a project with existing `.tala/config.json`
- **THEN** output SHALL be valid JSON: `{"name": "<project-name>", "already_initialized": true, "daemon_started": true}`

### Requirement: init auto-starts the daemon
`tala init` SHALL start the tala daemon automatically if it is not already running.

#### Scenario: init starts daemon on first run
- **WHEN** user runs `tala init` in a project with no running daemon
- **THEN** after writing config, tala starts the daemon
- **THEN** tala is immediately usable without running an additional command

#### Scenario: init does not error if daemon is already running
- **WHEN** user runs `tala init` and the daemon is already running
- **THEN** init completes successfully without duplicate daemon errors
