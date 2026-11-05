### Requirement: Unified wait observes board activity
`tala wait` SHALL observe new posts and replies by other agents on the selected project's board. It SHALL return event IDs and types without message bodies and SHALL not acknowledge board or inbox receipts. A board reply delivered to inbox SHALL appear once with both inbox and board sources.

#### Scenario: Wait returns new board activity
- **WHEN** an agent waits and another agent posts or replies on the selected board
- **THEN** Tala returns the new event ID and type without acknowledging it

### Requirement: Board event cursors
Board notifications SHALL advance only through messages in the returned board page and SHALL provide exact board-fetch and `listen_again` commands.

#### Scenario: Board event continuation
- **WHEN** an agent receives a board event notification
- **THEN** its board fetch and restart commands use the appropriate prior and returned cursors

### Requirement: Wait can watch an explicitly selected board
`tala wait --project=<path>` SHALL watch the selected board and include its canonical project path in continuation commands.

#### Scenario: Wait on an explicit board
- **WHEN** an agent runs `tala wait --project=<path> --board-after=<id>` and another agent posts after that ID
- **THEN** the new board message is returned as an event and the continuation retains the explicit project and returned cursor

### Requirement: New board waits establish a baseline
If `tala wait` starts without `--board-after`, the CLI SHALL use the board's atomic high-water mark as its baseline. A caller SHALL resume using the cursor returned in `next.listen_again`.

#### Scenario: Board baseline is returned for restart
- **WHEN** an agent starts `tala wait` without a board cursor and the board already has messages
- **THEN** those messages do not trigger an immediate notification and the returned continuation includes the startup baseline

### Requirement: Board pages expose a project high-water mark
Every board page SHALL include `latest_id`, the highest message ID on that project board. The high-water mark and page SHALL be read atomically.

#### Scenario: Board page exposes atomic project high-water mark
- **WHEN** an agent reads a limited board page while the project has newer messages beyond that page
- **THEN** the response includes `latest_id` equal to the highest message ID on that project board
