# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
