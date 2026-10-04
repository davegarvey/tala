## Purpose

Defines agent identity for reliable local collaboration between distinct coding agents in shared and separate projects.

## Requirements

### Requirement: Independent instances
The system SHALL register distinct agent IDs with name, tool, project checkout and context, even when names or checkouts match. Commands SHALL select identity only through --agent or TALA_AGENT, never project-local active state.

#### Scenario: Two agents share a checkout
- **WHEN** Claude and Codex register from one checkout
- **THEN** their IDs and inbox state differ and nested directories resolve to the same checkout

### Requirement: Discovery and presence
The system SHALL list registered agents with IDs, context, last_seen, inactivity and listening status. Exact IDs SHALL resolve uniquely; ambiguous names SHALL fail with candidate IDs. Unregister SHALL mark an instance inactive without deleting its history.

#### Scenario: Duplicate names
- **WHEN** two agents have the name codex
- **THEN** addressing codex fails with candidate IDs and an exact ID succeeds
