# Spec Delta

## ADDED Requirements

### Requirement: Project discovery without identity
The `list-projects` command SHALL allow read-only project discovery without a selected agent identity. Supplying a valid identity SHALL remain optional and preserve the authenticated response behavior.

#### Scenario: List projects before registration
- **WHEN** a user runs `tala list-projects` without `--agent` or `TALA_AGENT`
- **THEN** Tala returns the server-wide project list with the normal filters, pagination, and hidden-project counts

#### Scenario: Authenticated project listing
- **WHEN** a registered agent runs `tala list-projects` with its selected identity
- **THEN** Tala returns the same project listing and the agent's response includes its normal ambient attention summary
