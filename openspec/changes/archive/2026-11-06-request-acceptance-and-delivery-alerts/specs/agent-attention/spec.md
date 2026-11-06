# Spec Delta

## MODIFIED Requirements

### Requirement: Background wait notifications do not acknowledge
`tala wait` SHALL monitor unread inbox messages using a non-consuming read, including direct replies and board replies delivered to the agent. It SHALL advance its inbox notification cursor only through events returned to the caller and SHALL leave inbox receipt state unchanged. Continuation commands SHALL carry separate inbox and board cursors; replaying an older inbox cursor MAY repeat an event but SHALL NOT hide it or mark it received. Wait SHALL also return durable request lifecycle and delivery-overdue events using an identity-bound event cursor.

#### Scenario: Wait preserves unread receipts while returning request events
- **WHEN** an acceptance, decline, deadline, supersession or delivery-overdue event occurs while the sender is waiting
- **THEN** the blocking wait wakes with the event and leaves unrelated inbox receipts unchanged

#### Scenario: Resume after restart
- **WHEN** a wait resumes from an earlier event cursor after daemon restart
- **THEN** every unreturned durable event remains available in cursor order

#### Scenario: Replaying an older event cursor
- **WHEN** a caller repeats a wait using an older event cursor
- **THEN** events may repeat but none are lost or acknowledged by replay

#### Scenario: Wait notification preserves unread receipt
- **WHEN** `tala wait` returns an unread message notification
- **THEN** `tala inbox --peek` still returns that message as unread

#### Scenario: Lost wait output can safely replay
- **WHEN** an agent resumes with an earlier inbox cursor after losing a wait result
- **THEN** the notification may repeat
- **AND** the message remains available until explicitly consumed

## ADDED Requirements

### Requirement: Pending request states
Pending output SHALL distinguish active requests awaiting acceptance from accepted requests awaiting reply, and SHALL identify declined requests and expired acceptance deadlines without presenting expiry as execution failure.

#### Scenario: Pending shows commitment status
- **WHEN** a sender lists outgoing requests
- **THEN** each active request identifies whether it is awaiting acceptance or accepted and awaiting reply, with any deadline status

#### Scenario: Deadline is overdue but work is not failed
- **WHEN** an acceptance deadline has expired
- **THEN** pending shows the overdue acceptance while retaining the request as open

### Requirement: Receiver attention hints
When an agent has unread direct messages or an open incoming request and is not currently listening, attention output SHALL include a concise hint to start or resume `tala wait`. Inactive presence SHALL be described only as an availability observation.

#### Scenario: Unread mail while not listening
- **WHEN** a command reports unread mail and the agent has no active listening lease
- **THEN** its attention includes a `tala wait` hint

#### Scenario: Open request after its message was read
- **WHEN** an agent has an open incoming request but no unread messages and no active listening lease
- **THEN** its attention includes a `tala wait` hint
