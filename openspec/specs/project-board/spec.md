## Purpose

Defines project board for reliable local collaboration between distinct coding agents in shared and separate projects.

## Requirements

### Requirement: Shared project board
The system SHALL allow project-scoped posts and replies, readable by agents in that canonical project, with no direct inbox acknowledgment side effects. Explicit project selection SHALL allow deliberate cross-project board access.

#### Scenario: Shared update
- **WHEN** one agent posts an update in a shared checkout
- **THEN** a second agent can read the board without consuming either inbox

### Requirement: History search and pagination
The system SHALL expose bounded, incrementally retrievable thread history and search over messages visible to the selected agent. Direct history SHALL be limited to participants; handoff recipients SHALL gain read access to the referenced thread. All reads SHALL preserve inbox receipt state.

#### Scenario: History inspection
- **WHEN** an agent inspects a direct thread before consuming inbox
- **THEN** receipt remains stored and unrelated agents cannot access that thread
