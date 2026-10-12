# Spec Delta

## MODIFIED Requirements

### Requirement: Independent instances
The system SHALL register distinct agent IDs with name, tool, project checkout and context, even when names or checkouts match. Commands SHALL select identity only through a private handle given with --agent or TALA_AGENT, never project-local active state.

#### Scenario: Two agents share a checkout
- **WHEN** Claude and Codex register from one checkout
- **THEN** their IDs, handles and inbox state differ and nested directories resolve to the same checkout

## ADDED Requirements

### Requirement: Private identity handle
`register` SHALL return a private handle, distinct from the public agent ID, that is required to act as that agent. Public IDs SHALL be used only for addressing and display. Supplying a public ID to `--agent` or `TALA_AGENT` SHALL fail with `IDENTITY_ERROR` and a hint explaining the difference. Handles SHALL never appear in messages, listings or other agents' output.

#### Scenario: Public ID cannot select identity
- **WHEN** an agent passes another agent's `agt_` ID from `tala agents` to `--agent`
- **THEN** the command fails with `IDENTITY_ERROR` and no message is sent or consumed

#### Scenario: Handle not disclosed
- **WHEN** any agent lists agents, reads history or searches messages
- **THEN** no output contains another agent's handle

### Requirement: Existing identities upgrade explicitly
Identities created before handles existed SHALL remain addressable and keep their history. Their owner SHALL obtain a handle through an explicit `register --reissue=<agt_id>`, a transitional option hidden from help, which SHALL succeed at most once per identity and retire the ID-keyed credential. No handle SHALL be issued implicitly.

#### Scenario: Reissue for an existing identity
- **WHEN** an agent with a protocol 2 identity runs `register --reissue=<its id>`
- **THEN** it receives a handle for the same agent ID and its existing inbox remains intact

#### Scenario: Second reissue refused
- **WHEN** `register --reissue` runs again for an identity that already has a handle
- **THEN** it fails with `IDENTITY_ERROR` and the existing handle remains valid
