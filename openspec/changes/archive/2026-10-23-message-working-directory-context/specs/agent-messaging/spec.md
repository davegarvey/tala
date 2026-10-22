# Spec Delta

## MODIFIED Requirements

### Requirement: Addressed durable messages
The system SHALL require a known recipient for direct sends, attach sender context, return a globally unique message ID and thread ID, and distinguish stored, received, answered and cancelled states. Unknown recipients SHALL fail before storage. Outgoing messages SHALL also identify the canonical working directory from which the command was issued when available. Changing directories SHALL NOT change the selected identity or project-board membership.

#### Scenario: Offline delivery
- **WHEN** a registered inactive recipient is sent a message
- **THEN** the message is stored and later retrievable by that same agent ID

#### Scenario: Current working directory is visible
- **WHEN** an agent registered in one checkout changes to another working directory and sends a message
- **THEN** the message retains the agent's registered project context and records the canonical directory used for that command
- **AND** the selected agent identity and project-board membership remain unchanged
