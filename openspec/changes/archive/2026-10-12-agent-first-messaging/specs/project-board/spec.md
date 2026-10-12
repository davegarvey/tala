# Spec Delta

## MODIFIED Requirements

### Requirement: Shared project board
The system SHALL allow project-scoped posts and replies, readable by agents in that canonical project, with no direct inbox acknowledgment side effects. The canonical project of a git checkout SHALL be its repository's main worktree root, shared by all linked worktrees; each message SHALL still record its own worktree, branch and revision. Explicit project selection SHALL allow deliberate cross-project board access.

#### Scenario: Shared update
- **WHEN** one agent posts an update in a shared checkout
- **THEN** a second agent can read the board without consuming either inbox

#### Scenario: Sibling worktrees
- **WHEN** an agent in one linked worktree posts an update
- **THEN** an agent registered in another worktree of the same repository reads it on its default board, with the poster's worktree path shown

#### Scenario: Unrelated repositories
- **WHEN** agents register in two different repositories
- **THEN** their default boards are separate
