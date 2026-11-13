# Spec Delta

## ADDED Requirements

### Requirement: Registration distinguishes private credentials from public addresses
Registration output and the identity guide SHALL clearly explain that the returned `th_` handle is a private credential and that public `agt_` IDs are used to address agents.

#### Scenario: Registration explains handle privacy
- **WHEN** an agent registers and receives a private handle
- **THEN** the human-readable output identifies it as a private credential and explains that public `agt_` IDs are used for addressing

#### Scenario: Identity guide explains handle privacy
- **WHEN** an agent reads the identity guide
- **THEN** it clearly distinguishes the private `th_` credential from public `agt_` addressing IDs
