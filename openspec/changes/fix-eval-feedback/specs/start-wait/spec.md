## ADDED Requirements

### Requirement: start --wait flag
`tala start` SHALL accept a `--wait` / `-w` flag that, when set, blocks the command until at least one reply is received in the newly created session.

#### Scenario: start with --wait flag
- **WHEN** user runs `tala start --wait`
- **THEN** a new session is created and tala prints the session ID
- **THEN** tala waits for a reply and prints it when received

#### Scenario: start with --wait flag times out
- **WHEN** user runs `tala start --wait --timeout 10`
- **THEN** a new session is created
- **THEN** tala waits up to 10 seconds for a reply
- **THEN** if no reply arrives, tala prints a timeout message and exits with code 2

#### Scenario: start with --wait and --json
- **WHEN** user runs `tala start --wait --json`
- **THEN** output SHALL be valid JSON containing `session_id` and either `messages` or `timeout` fields

### Requirement: start --wait timeout flag
`tala start --wait` SHALL accept a `--timeout` / `-t` flag to override the default wait timeout (in seconds).

#### Scenario: custom timeout
- **WHEN** user runs `tala start --wait --timeout 120`
- **THEN** tala waits up to 120 seconds for a reply before timing out

### Requirement: start --wait reuses existing wait machinery
`tala start --wait` SHALL reuse the existing `/api/sessions/{id}/wait` daemon endpoint rather than introducing a new API.

#### Scenario: wait endpoint called after session creation
- **WHEN** user runs `tala start --wait`
- **THEN** after creating the session, tala calls the existing wait endpoint with the session ID
