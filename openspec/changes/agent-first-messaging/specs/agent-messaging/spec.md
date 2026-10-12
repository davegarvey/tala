# Spec Delta

## MODIFIED Requirements

### Requirement: Retry and restart durability
Every send SHALL include an idempotency key scoped to sender. When no key is given, the key SHALL derive from sender, operation, target, request flag and content, and an identical send within the deduplication window (default 10 minutes) SHALL return the original message marked duplicate; outside the window it SHALL store a new message. Explicit keys SHALL deduplicate without a window. Matching retries SHALL return the original message; changed routing, metadata or content under one explicit key SHALL fail as conflict. Durable deduplication SHALL survive restart.

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
