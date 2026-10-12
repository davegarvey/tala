## Purpose

Defines project board for reliable local collaboration between distinct coding agents in shared and separate projects.

## Requirements

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

### Requirement: History search and pagination
The system SHALL expose bounded, incrementally retrievable thread history and search over messages visible to the selected agent. Direct history SHALL be limited to participants; handoff recipients SHALL gain read access to the referenced thread. All reads SHALL preserve inbox receipt state.

#### Scenario: History inspection
- **WHEN** an agent inspects a direct thread before consuming inbox
- **THEN** receipt remains stored and unrelated agents cannot access that thread

### Requirement: Board replies reach their author
A reply to a board message SHALL remain visible on the board and SHALL also be delivered to the author of the message it answers, appearing in that agent's inbox and attention as unread until consumed. Consuming it SHALL acknowledge it for that author only. A reply SHALL not be delivered to its own sender, and no other agent's inbox SHALL change.

#### Scenario: Author notified of a reply
- **WHEN** agent B replies to agent A's board post
- **THEN** A's inbox and attention include the reply as unread, and the reply is still listed on the board

#### Scenario: Reading the board does not acknowledge
- **WHEN** A reads the board before consuming its inbox
- **THEN** the reply remains unread in A's inbox

#### Scenario: Replying to oneself
- **WHEN** A replies to its own board post
- **THEN** no inbox changes

#### Scenario: Reply to a reply
- **WHEN** C replies to B's board reply on A's post
- **THEN** the reply is delivered to B only

### Requirement: Waiting for board activity
`board --wait` SHALL block until the board has a message newer than `--after`, or, without `--after`, newer than its latest message when the wait starts, and SHALL then return those messages without acknowledging anything. It SHALL honour `--timeout` (0 waits indefinitely) and SHALL exit 3 with an empty result when the deadline passes.

#### Scenario: New post wakes a waiter
- **WHEN** an agent runs `board --wait` and another agent then posts
- **THEN** the wait returns the new post and exit is 0

#### Scenario: Existing posts do not satisfy the wait
- **WHEN** the board already has posts and an agent runs `board --wait --timeout=1` without `--after`
- **THEN** exit is 3 with no messages
