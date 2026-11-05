## Purpose

Defines agent messaging for reliable local collaboration between distinct coding agents in shared and separate projects.

## Requirements

### Requirement: Addressed durable messages
The system SHALL require a known recipient for direct sends, attach sender context, return a globally unique message ID and thread ID, and distinguish stored, received, answered and cancelled states. Unknown recipients SHALL fail before storage. Outgoing messages SHALL identify the canonical working directory and Git worktree, branch and revision from which the command was issued when available. Changing directories SHALL NOT change the selected identity or project-board membership.

#### Scenario: Offline delivery
- **WHEN** a registered inactive recipient is sent a message
- **THEN** the message is stored and later retrievable by that same agent ID

#### Scenario: Current working directory is visible
- **WHEN** an agent registered in one checkout changes to another working directory and sends a message
- **THEN** the message retains the agent's registered project context and records the canonical directory used for that command
- **AND** the message's Git worktree, branch and revision reflect the invocation directory when it is in a Git worktree
- **AND** the selected agent identity and project-board membership remain unchanged

#### Scenario: Git context is absent outside a worktree
- **WHEN** an agent registered in a Git checkout sends a message from a directory outside any Git worktree
- **THEN** the message retains the registered project context and records no worktree, branch or revision

### Requirement: Replies and obligations
The system SHALL route reply automatically to the original sender within its thread and permit replies only by the addressed recipient. Requests SHALL remain pending after receipt until explicitly replied to, cancelled by their sender, or superseded by a later direct message from that sender to the same recipient in the same thread. A follow-up request SHALL not settle another request. A recipient SHALL NOT reply to a superseded request; the error SHALL identify the superseding message.

#### Scenario: Receipt is not an answer
- **WHEN** a recipient consumes a request
- **THEN** delivery becomes received but the request remains pending

#### Scenario: Superseding a request clears its obligation
- **WHEN** a sender posts a direct message that supersedes its unanswered request to the same recipient in the same thread
- **THEN** the older request is no longer pending
- **AND** both messages remain visible in the thread with their relationship

#### Scenario: Reply to superseded request is rejected
- **WHEN** a recipient replies to a superseded request
- **THEN** Tala returns an actionable error naming the successor message ID
- **AND** no reply message is stored

### Requirement: Inbox isolation and acknowledgment
The system SHALL return only the selected agent inbox. Consumption SHALL atomically acknowledge only returned messages; peek SHALL not acknowledge. Pages SHALL expose next_after and has_more.

#### Scenario: Concurrent consumption
- **WHEN** two commands consume one agent inbox simultaneously
- **THEN** each unread message is consumed at most once and other agent inboxes are unchanged

### Requirement: Retry and restart durability
Every send SHALL include an idempotency key scoped to sender; without an explicit key it SHALL derive from sender, routing and content. Derived keys SHALL deduplicate identical sends within 10 minutes; explicit keys SHALL deduplicate without a window. Matching retries SHALL return the original message marked duplicate; changed routing, metadata or content under one explicit key SHALL fail as conflict. Durable deduplication SHALL survive restart.

#### Scenario: Key conflict
- **WHEN** one explicit key is reused with another recipient
- **THEN** the second operation fails without another message

#### Scenario: Blind resend
- **WHEN** an agent repeats an identical send without a key two minutes after the first
- **THEN** the result returns the original message ID with duplicate true and no new message is stored

#### Scenario: Deliberate repeat later
- **WHEN** an agent sends identical text without a key after the deduplication window
- **THEN** a new message is stored

#### Scenario: Same text from different agents
- **WHEN** two agents in one checkout send identical text to the same recipient without keys
- **THEN** two messages are stored

### Requirement: Direct-message recipient listening observation
Direct send and direct reply results SHALL include `recipient_presence.listening`, a boolean captured from the resolved recipient's active listening lease when Tala accepts the message. This is a point-in-time observation and SHALL NOT imply that the recipient will respond. Matching idempotent retries SHALL return the original observation. Board posts and board replies SHALL omit this field.

#### Scenario: Recipient is listening
- **WHEN** a direct message is accepted while its active recipient has a current listening lease
- **THEN** the result includes `recipient_presence.listening: true`

#### Scenario: Recipient is not listening
- **WHEN** a direct message is accepted while its recipient is inactive or has no current listening lease
- **THEN** the result includes `recipient_presence.listening: false` and the message remains queued for later retrieval

#### Scenario: Direct reply observes original sender
- **WHEN** a direct reply is accepted
- **THEN** its presence observation describes the original sender, who is the reply recipient

#### Scenario: Idempotent retry preserves observation
- **WHEN** an identical direct send or reply is retried after the recipient's listening state changes
- **THEN** the result returns the original message and original `recipient_presence.listening` value

#### Scenario: Board activity has no recipient presence
- **WHEN** a board post or board reply is accepted
- **THEN** the result omits `recipient_presence`

### Requirement: Handoff and blocking reply
The system SHALL send a handoff summary referencing a visible thread to an explicit recipient. Blocking send SHALL wait only for a correlated reply, retry transient transport failures until its deadline, return stored request details on timeout, and preserve eventual late replies. If the wait fails permanently after storage, it SHALL report the stored message ID and a command to resume waiting.

#### Scenario: Late reply
- **WHEN** send --wait times out before its recipient replies
- **THEN** exit is 3 with the stored request and the late reply remains in the sender inbox

#### Scenario: Interrupted wait recovers
- **WHEN** the daemon connection drops for two seconds during send --wait and the reply arrives afterwards
- **THEN** the wait returns the reply with exit 0

#### Scenario: Wait fails after storage
- **WHEN** the daemon stays unreachable for the rest of send --wait's deadline after storing the request
- **THEN** exit is 4, `stored` holds the request ID and `next` is a command that waits for its reply without resending

### Requirement: Non-disclosing thread visibility errors
Operations that reference a thread outside the selected identity's visible scope SHALL return `THREAD_NOT_VISIBLE`. This failure SHALL use the same public message and hint for inaccessible, missing, and malformed thread IDs, SHALL NOT reveal handles or foreign project metadata, and SHALL set `next` to `tala guide identity`.

#### Scenario: History references an inaccessible thread
- **WHEN** an agent requests history for a missing, malformed, or out-of-scope thread ID
- **THEN** Tala returns the generic `THREAD_NOT_VISIBLE` failure with identical public fields for both cases

#### Scenario: Handoff references an inaccessible thread
- **WHEN** an agent hands off a thread outside its visible scope
- **THEN** Tala returns the same non-disclosing `THREAD_NOT_VISIBLE` failure as history

#### Scenario: Another thread operation references an inaccessible thread
- **WHEN** an agent tries to continue, inspect, or reply within a thread outside its visible scope
- **THEN** Tala returns the same non-disclosing `THREAD_NOT_VISIBLE` failure

### Requirement: Direct-message amendment relationships
The system SHALL allow a direct message to supersede one earlier direct message only when both messages have the same sender, resolved recipient, and thread. The earlier message and its read/answer state SHALL be preserved. A message SHALL have at most one successor, while a successor MAY itself be superseded to form a linear chain. Board messages and direct handoffs SHALL NOT be superseded. Message output SHALL expose `supersedes` on a successor and `superseded_by` on its predecessor.

Supersession SHALL update message metadata and request obligations without interrupting work already underway. Agents SHOULD check for newer instructions at work-increment boundaries. An unread superseded message SHALL remain unread and visible with its successor relationship.

#### Scenario: A direct message supersedes an earlier message
- **WHEN** a sender sends a direct message with `--supersedes=<id>` to the same recipient in the same thread
- **THEN** the new message identifies the earlier message in `supersedes`
- **AND** the earlier message identifies the new message in `superseded_by`
- **AND** neither message's receipt or answer timestamps are changed by the relationship

#### Scenario: Thread is inferred from superseded message
- **WHEN** the sender omits `--thread` while superseding a message
- **THEN** Tala places the new message in the earlier message's thread

#### Scenario: Supersession chain is linear
- **WHEN** a sender supersedes the current latest message in a chain
- **THEN** the new message extends that chain
- **AND** an earlier message that already has a successor cannot receive a second successor

#### Scenario: Supersession cannot preempt in-flight work
- **WHEN** a direct message supersedes an instruction that its recipient is already acting on
- **THEN** Tala records the relationship and updates request obligations without interrupting the recipient's current work
- **AND** agents can check for the newer instruction at a work-increment boundary

#### Scenario: Supersession target is outside the allowed direct thread
- **WHEN** a sender targets a board post, a handoff, another sender's message, a message to another recipient, or a message in another thread
- **THEN** Tala rejects the send without storing a message

#### Scenario: Supersession target is not earlier or does not exist
- **WHEN** a sender targets an unknown, self, or future message ID
- **THEN** Tala rejects the send without storing a message

### Requirement: Supersession retries are durable and idempotent
The system SHALL include a supersession target in message canonicalization and explicit-key conflict detection. Derived retry keys SHALL distinguish sends with different supersession targets. When no supersession target is supplied, retry identity SHALL remain compatible with existing sends.

#### Scenario: Matching superseding retry
- **WHEN** an identical superseding send is retried with the same sender and idempotency key
- **THEN** Tala returns the original message and its original amendment relationship without creating another successor

#### Scenario: Changed supersession target conflicts
- **WHEN** a retry reuses an explicit idempotency key with a different supersession target
- **THEN** Tala returns `IDEMPOTENCY_CONFLICT` and stores no message

#### Scenario: Existing ordinary-send retry remains compatible
- **WHEN** an ordinary send without `--supersedes` is retried after upgrading storage
- **THEN** the send retains its existing explicit and derived idempotency behavior
