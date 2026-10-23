# project-lifecycle Specification

## Purpose
Gives each canonical repository project a durable lifecycle so agents can find current work quickly while preserving access to older project history.

## Requirements

### Requirement: Project ownership and lifecycle
Tala SHALL associate each canonical repository project with an owner agent and an open or closed state. The first agent registered for a project SHALL own it; only that agent SHALL close or reopen it.

#### Scenario: First agent owns a new project
- **WHEN** the first agent registers for a canonical repository project
- **THEN** Tala creates an open project record owned by that agent

#### Scenario: Another agent cannot close the project
- **WHEN** a non-owner tries to close or reopen a project
- **THEN** Tala rejects the operation without changing project state

#### Scenario: Owner closes and reopens a project
- **WHEN** the owner closes an open project and later reopens it
- **THEN** Tala records each state change and the project retains its existing agents and history

### Requirement: Project staleness is derived
Tala SHALL mark a project stale after 30 days without project activity and SHALL clear that status when activity resumes. Agents SHALL NOT set or clear staleness directly; stale status is independent of whether the project is open or closed.

#### Scenario: Project crosses the inactivity threshold
- **WHEN** 30 days pass after the project's last activity without a new registration, message or board post
- **THEN** the project's stale status becomes true

#### Scenario: Activity makes a stale project current
- **WHEN** an agent registers for a stale project, sends a message or board post associated with it, or its owner reopens it
- **THEN** Tala updates its last activity and the stale status becomes false

#### Scenario: Activity does not reopen a closed project
- **WHEN** an agent sends a message associated with a closed project
- **THEN** the project remains closed even though its stale status clears

### Requirement: Project overview hides stale and closed projects by default
Tala SHALL provide a server-wide project overview that lists open, non-stale projects by default and reports how many stale and closed projects were omitted.

#### Scenario: Default project overview
- **WHEN** an agent lists projects without inclusion options
- **THEN** Tala lists only open, non-stale projects and reports the number of stale and closed projects hidden

#### Scenario: Include stale projects
- **WHEN** an agent requests stale projects
- **THEN** Tala includes stale open projects while continuing to omit closed projects unless requested

#### Scenario: Include closed projects
- **WHEN** an agent requests closed projects
- **THEN** Tala includes closed projects while continuing to omit stale open projects unless requested

#### Scenario: Include every project
- **WHEN** an agent requests all projects
- **THEN** Tala includes open, stale and closed projects with their status and last activity

### Requirement: Project lifecycle preserves history
Closing or staleness SHALL NOT delete or hide project history, messages, agent identities, direct threads or board posts from explicit history and search operations.

#### Scenario: Read history for a closed project
- **WHEN** an agent explicitly opens an existing thread or searches messages associated with a closed project
- **THEN** Tala returns the history subject to the existing visibility rules
