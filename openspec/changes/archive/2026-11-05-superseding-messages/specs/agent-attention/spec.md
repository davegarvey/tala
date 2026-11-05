# Spec Delta

## ADDED Requirements

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
