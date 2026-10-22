## Purpose

Defines agent messaging for reliable local collaboration between distinct coding agents in shared and separate projects.

## Requirements

### Requirement: Addressed durable messages
The system SHALL require a known recipient for direct sends, attach sender context, return a globally unique message ID and thread ID, and distinguish stored, received, answered and cancelled states. Unknown recipients SHALL fail before storage. Outgoing messages SHALL also identify the canonical working directory from which the command was issued when available. Changing directories SHALL NOT change the selected identity or project-board membership.

#### Scenario: Offline delivery
- **WHEN** a registered inactive recipient is sent a message
- **THEN** the message is stored and later retrievable by that same agent ID

#### Scenario: Current working directory is visible
- **WHEN** an agent registered in one checkout changes to another working directory and sends a message
- **THEN** the message retains the agent's registered project context and records the canonical directory used for that command
- **AND** the selected agent identity and project-board membership remain unchanged

### Requirement: Replies and obligations
The system SHALL route reply automatically to the original sender within its thread and permit replies only by the addressed recipient. Requests SHALL remain pending after receipt until explicitly replied to or cancelled by their sender. A follow-up request SHALL not settle another request.

#### Scenario: Receipt is not an answer
- **WHEN** a recipient consumes a request
- **THEN** delivery becomes received but the request remains pending

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
