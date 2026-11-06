# Spec Delta

## MODIFIED Requirements

### Requirement: Replies and obligations
The system SHALL route reply automatically to the original sender within its thread and permit replies only by the addressed recipient. Requests SHALL remain pending after receipt until explicitly replied to, cancelled by their sender, or superseded by a later direct message from that sender to the same recipient in the same thread. A follow-up request SHALL not settle another request. A recipient SHALL NOT reply to a superseded request; the error SHALL identify the superseding message. An explicit acceptance SHALL record a commitment separately from reply, and reply SHALL complete the request whether or not it was accepted.

#### Scenario: Delivery does not imply acceptance
- **WHEN** a recipient fetches a request without accepting it
- **THEN** it remains pending and unaccepted

#### Scenario: Reply completes an unaccepted request
- **WHEN** the addressed recipient replies before accepting
- **THEN** the request is answered and no acceptance deadline alert is emitted afterward

#### Scenario: Accepted request is superseded
- **WHEN** a sender supersedes a request after the recipient accepted it
- **THEN** the old request is no longer pending and the sender receives a durable superseded event

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

### Requirement: Request acceptance decisions
The addressed recipient SHALL be able to accept or decline an active request. Repeating the same decision SHALL be idempotent and SHALL NOT create another event; conflicting decisions SHALL fail. Decisions SHALL NOT reopen answered, cancelled, or superseded requests.

#### Scenario: Accept a request once
- **WHEN** the recipient accepts an active request
- **THEN** the request records acceptance and its sender can observe one acceptance event

#### Scenario: Decline a request once
- **WHEN** the recipient declines an active request with an optional reason
- **THEN** the request records decline and its sender can observe one decline event

#### Scenario: Sender or other agent attempts a decision
- **WHEN** an agent other than the addressed recipient accepts or declines
- **THEN** the operation fails without changing request state

### Requirement: Acceptance deadlines
Senders MAY set an acceptance deadline when creating a request. Expiry SHALL alert the sender without marking the request failed, cancelling it, or preventing later acceptance. Answering, cancellation, or supersession before expiry SHALL suppress the expiry alert.

#### Scenario: Deadline expires without acceptance
- **WHEN** an active request with an opt-in deadline remains unaccepted past its deadline
- **THEN** one durable deadline event becomes visible and the request remains open

#### Scenario: Request settles before its deadline
- **WHEN** a request is answered, cancelled, or superseded before expiry
- **THEN** no deadline event is created

#### Scenario: Late acceptance after expiry
- **WHEN** the recipient accepts after the sender was alerted about expiry
- **THEN** acceptance succeeds and is visible as a separate event
