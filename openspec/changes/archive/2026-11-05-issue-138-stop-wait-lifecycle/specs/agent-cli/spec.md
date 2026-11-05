## MODIFIED Requirements

### Requirement: Exit codes
The CLI SHALL exit 0 on success, 1 on operational error with nothing stored, 2 on usage error, 3 on wait timeout, 4 when a message was stored but a subsequent wait failed, and 5 when an active wait is ended by an accepted daemon stop. Help output SHALL list these codes. A `DAEMON_STOPPED` result SHALL be terminal to a supervisor and SHALL NOT cause automatic rearming. When `send --wait` times out after storing its request, its result SHALL identify the stored request and provide one exact command to resume waiting for that request without resending it.

#### Scenario: Active wait is stopped
- **WHEN** `tala wait` or `tala inbox --wait` is active when an accepted stop advances the lifecycle generation
- **THEN** it exits 5 with `DAEMON_STOPPED` and does not start or adopt a replacement daemon

#### Scenario: New wait begins during shutdown
- **WHEN** a new wait command starts while lifecycle state marks an instance as stopping
- **THEN** it exits 5 immediately without waiting for the old daemon or starting a replacement

#### Scenario: Wait event races with stop
- **WHEN** stop is accepted before a non-consuming wait accepts its event result
- **THEN** the wait exits 5 rather than returning the event
- **WHEN** the event is accepted before stop
- **THEN** the wait may return the event with exit 0 and the next wait invocation exits 5 if shutdown is still active

#### Scenario: Inbox receipt survives stop race
- **WHEN** `inbox --wait` consumes and acknowledges a nonempty batch before or during a racing stop
- **THEN** it returns the consumed messages with exit 0
- **AND** a supervisor's next wait invocation exits 5 if shutdown is still active

#### Scenario: Stored then failed
- **WHEN** `send --wait` stores its request and the wait then fails permanently
- **THEN** exit is 4 and the error's `stored` field holds the message ID

#### Scenario: Stored request times out
- **WHEN** `send --wait --timeout=N` stores its request but receives no correlated reply before the deadline
- **THEN** it exits 3, reports that the request is stored and timed out, and provides a `next` command using `inbox --wait --reply-to=<message-id> --timeout=N`
- **AND** the command does not suggest resending the request

### Requirement: Unified background wait
The CLI SHALL expose `tala wait` to monitor unread inbox messages and new messages on the selected project's board. Its timeout SHALL default to indefinite; finite `--timeout` SHALL remain available. Inbox and board reads SHALL not acknowledge messages. A new wait without `--board-after` SHALL use the project's atomic latest-message ID as its baseline. A wait SHALL bind to its lifecycle generation and SHALL exit 5 with `DAEMON_STOPPED` if an accepted stop advances that generation.

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

#### Scenario: Wait stops instead of recovering
- **WHEN** an accepted stop advances the waiter's captured lifecycle generation
- **THEN** the wait exits 5 with `DAEMON_STOPPED` without recovering the daemon

### Requirement: Wait cursors and recovery
The command SHALL accept separate inbox and board ID cursors, return event IDs and types without message bodies, and provide exact fetch and restart commands. It SHALL report whether either source has another page. Each cursor SHALL advance only to the last ID included in its returned page. Replaying an older cursor MAY repeat events but SHALL NOT hide or acknowledge messages. A supervisor SHALL treat exit 5 / `DAEMON_STOPPED` as a terminal stop signal and SHALL NOT rearm the wait loop.

#### Scenario: Supervisor stops after daemon stop
- **WHEN** a wait loop receives exit 5 with `DAEMON_STOPPED`
- **THEN** it exits without issuing another wait command

#### Scenario: Finite wait timeout
- **WHEN** `tala wait --timeout=N` receives no new event before its deadline
- **THEN** it exits 3 with no events and exact continuation commands using the unchanged cursors

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
