## Purpose

Tells an agent, in one bounded call, which messages and requests need its action and how to act on each, without burying current work under stale history.

## Requirements

### Requirement: Attention summary
Running `tala` with no subcommand and a selected identity SHALL return, without acknowledging any message: incoming unanswered requests oldest first, the count and first page of unread messages, and the agent's own outstanding requests. Each item SHALL include an exact `next` command. The summary SHALL be bounded by the standard list limit.

#### Scenario: Request waiting
- **WHEN** an agent with one unanswered incoming request and two unread messages runs `tala`
- **THEN** the result lists the request with a `next` reply command, reports two unread messages, and a following `tala inbox --peek` still shows them unread

#### Scenario: Nothing to do
- **WHEN** an agent with no unread messages and no open requests runs `tala`
- **THEN** the result has empty lists and exit code 0

#### Scenario: No identity selected
- **WHEN** `tala` runs without `--agent` or `TALA_AGENT`
- **THEN** it fails with `IDENTITY_REQUIRED` and a `next` command that registers an identity

### Requirement: Ambient attention
Every successful response to a command run with a selected identity SHALL include an `attention` field, computed in the same transaction as the command, holding the unread count, up to three oldest non-stale incoming unanswered request IDs with sender and `age_seconds`, the stale request count and a `next` command. The field SHALL be omitted when all counts are zero, SHALL contain no message bodies and SHALL acknowledge nothing.

#### Scenario: Learning of a request while posting
- **WHEN** an agent with one unanswered incoming request runs `tala post`
- **THEN** the response includes the posted message and an `attention` field naming the request, and the request remains unread

#### Scenario: Nothing waiting
- **WHEN** an agent with no unread messages and no open requests runs any command
- **THEN** the response has no `attention` field

#### Scenario: Reflects the command's own effect
- **WHEN** an agent consumes its last unread message with `tala inbox`
- **THEN** the same response's `attention` field reports no unread messages

### Requirement: Stale requests
A request SHALL be stale when it is older than 24 hours or its counterpart agent is unregistered. The attention summary and `pending` SHALL report stale requests as a count with the command to list them, and `pending --stale` SHALL list them. Staleness SHALL not change request state.

#### Scenario: Old request collapsed
- **WHEN** an agent has one fresh and twelve 30-day-old unanswered incoming requests
- **THEN** the summary lists the fresh request and reports twelve stale requests with `tala pending --stale` as the next command

#### Scenario: Stale request still answerable
- **WHEN** an agent replies to a stale request
- **THEN** the request becomes answered as normal

### Requirement: Machine-readable age
Messages and requests in JSON output SHALL include `age_seconds` computed by the daemon at response time alongside the stored creation timestamp. The text renderer (`--format=text`) SHALL show ages in days, hours and minutes.

#### Scenario: Age reported
- **WHEN** a request created 90 minutes ago is listed with JSON output
- **THEN** its `age_seconds` is approximately 5400

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


### Requirement: Superseded requests are not active obligations
The attention summary, ambient attention, `pending`, and stale-request counts SHALL exclude a request once it has a successor. Supersession SHALL NOT acknowledge or remove either message from the inbox or thread history; unread messages remain unread until the recipient explicitly consumes them.

#### Scenario: Superseded request leaves attention and pending
- **WHEN** an unanswered direct request is superseded by a message from its sender
- **THEN** it no longer appears in incoming or outgoing pending requests, attention request lists, or stale-request counts
- **AND** the successor remains visible as a new message

#### Scenario: Supersession does not acknowledge the prior message
- **WHEN** an unread message is superseded before its recipient consumes it
- **THEN** its receipt state remains unread
- **AND** inbox and history output identify the successor message

### Requirement: Pending request states
Pending output SHALL distinguish active requests awaiting acceptance from accepted requests awaiting reply, and SHALL identify overdue acceptance deadlines without presenting expiry as execution failure. Declined, answered, cancelled, and superseded requests SHALL not count as actionable pending work or attention. Inactive sender requests SHALL remain open and stale according to existing rules; inactivity alone SHALL NOT expire them.

#### Scenario: Pending shows commitment status
- **WHEN** a sender lists outgoing requests
- **THEN** each active request identifies whether it is awaiting acceptance or accepted and awaiting reply, with any deadline status

#### Scenario: Deadline is overdue but work is not failed
- **WHEN** an acceptance deadline has expired
- **THEN** pending shows the overdue acceptance while retaining the request as open

#### Scenario: Declined request leaves actionable attention
- **WHEN** a recipient declines a request
- **THEN** it no longer appears in incoming or outgoing pending requests, attention request lists, or stale-request counts
- **AND** its recorded decision remains available in history

#### Scenario: Inactive sender request remains open
- **WHEN** a sender becomes inactive while its request is unanswered
- **THEN** the request remains open and is marked stale rather than automatically expired

### Requirement: Receiver attention hints
When an agent has unread direct messages or an open incoming request and is not currently listening, attention output SHALL include a concise hint to start or resume `tala wait`. Inactive presence SHALL be described only as an availability observation.

#### Scenario: Unread mail while not listening
- **WHEN** a command reports unread mail and the agent has no active listening lease
- **THEN** its attention includes a `tala wait` hint

#### Scenario: Open request after its message was read
- **WHEN** an agent has an open incoming request but no unread messages and no active listening lease
- **THEN** its attention includes a `tala wait` hint
