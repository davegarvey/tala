## Purpose

Defines agent messaging for reliable local collaboration between distinct coding agents in shared and separate projects.

## ADDED Requirements

### Requirement: Addressed durable messages
The system SHALL require a known recipient for direct sends, attach sender context, return a globally unique message ID and thread ID, and distinguish stored, received, answered and cancelled states. Unknown recipients SHALL fail before storage.

#### Scenario: Offline delivery
- **WHEN** a registered inactive recipient is sent a message
- **THEN** the message is stored and later retrievable by that same agent ID

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
Every send SHALL include an idempotency key scoped to sender. Matching retries SHALL return the original message; changed routing, metadata or content SHALL fail as conflict. Durable deduplication SHALL survive restart.

#### Scenario: Key conflict
- **WHEN** one key is reused with another recipient
- **THEN** the second operation fails without another message

### Requirement: Handoff and blocking reply
The system SHALL send a handoff summary referencing a visible thread to an explicit recipient. Blocking send SHALL wait only for a correlated reply, return stored request details on timeout, and preserve eventual late replies.

#### Scenario: Late reply
- **WHEN** send --wait times out before its recipient replies
- **THEN** exit is 3 with the stored request and the late reply remains in the sender inbox
