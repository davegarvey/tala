### Requirement: Unified background wait
The CLI SHALL expose `tala wait` to monitor unread inbox messages and new messages on the selected project's board. Its timeout SHALL default to indefinite; finite `--timeout` SHALL remain available. Inbox and board reads SHALL not acknowledge messages. A new wait without `--board-after` SHALL use the project's atomic latest-message ID as its baseline.

#### Scenario: Wait for direct messages and correlated replies
- **WHEN** an agent runs `tala wait` and an unread direct message or reply arrives
- **THEN** the command returns the event ID and type without its body and leaves it unread
- **AND** its next commands fetch the inbox and resume with the returned inbox and board cursors

#### Scenario: New wait board baseline
- **WHEN** a new `tala wait` starts without `--board-after` while the board already contains messages
- **THEN** existing board messages do not immediately wake it
- **AND** the returned continuation command carries the baseline cursor

#### Scenario: Daemon lacks the atomic board high-water mark
- **WHEN** `tala wait` receives a board response without `latest_id`
- **THEN** it returns an actionable protocol-compatibility error instead of using a partial page cursor

### Requirement: Wait cursors and recovery
The command SHALL accept separate inbox and board ID cursors, return event IDs and types without message bodies, and provide exact fetch and restart commands. It SHALL report whether either source has another page. Each cursor SHALL advance only to the last ID included in its returned page. Replaying an older cursor MAY repeat events but SHALL NOT hide or acknowledge messages.

#### Scenario: Wait for board posts and replies
- **WHEN** an agent waits on its default board or an explicit `--project` board and a new message from another agent appears
- **THEN** the command returns its ID and event type without acknowledging it
- **AND** a board reply also present in the inbox appears once with both event sources

#### Scenario: Resume without skipping an inbox page
- **WHEN** more unread messages exist than fit in one wait result
- **THEN** the result's inbox cursor is the last ID actually returned
- **AND** the exact continuation command returns the remaining unread messages

#### Scenario: Restart after a lost initial wait
- **WHEN** an agent stops a brand-new wait before receiving its first result and starts another wait without a saved board cursor
- **THEN** the new wait establishes a fresh board baseline, while the intervening board history remains available to `tala board`

#### Scenario: Finite wait timeout
- **WHEN** `tala wait --timeout=N` receives no new event before its deadline
- **THEN** it exits 3 with no events and exact continuation commands using the unchanged cursors
