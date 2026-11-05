# Spec Delta

## ADDED Requirements

### Requirement: Thread-list visibility is identity-scoped
The thread list SHALL include direct threads where the selected identity sent or received a non-handoff message, or received a direct handoff grant. It SHALL include board threads only when that identity authored a board post or reply; same-project or `--project` board visibility alone SHALL NOT include a thread.

#### Scenario: Direct conversational participant
- **WHEN** an identity sent or received a non-handoff message in a direct thread
- **THEN** the thread appears with its conversational participants

#### Scenario: Direct handoff-only recipient
- **WHEN** an identity has only received a direct handoff grant for a thread
- **THEN** the thread appears and that identity is listed separately as a handoff-only recipient

#### Scenario: Board author or replier
- **WHEN** an identity authored a post or reply in a board thread
- **THEN** that thread appears in its list

#### Scenario: Same-project board bystander
- **WHEN** an identity can browse a board through its project context but did not author a post or reply in a board thread
- **THEN** that thread is absent from its history list

#### Scenario: New identity has no visible threads
- **WHEN** an identity has no conversational participation or direct handoff grants
- **THEN** the history list is empty

### Requirement: Thread-list attention uses canonical state without acknowledgment
Thread summaries SHALL count unread messages and pending requests using Tala's canonical delivery and request rules, without acknowledging or changing any delivery receipt.

#### Scenario: Direct unread messages include handoffs
- **WHEN** an identity has unreceived direct messages addressed to it, including handoff messages
- **THEN** the thread's unread count includes those messages

#### Scenario: Board unread deliveries
- **WHEN** an identity has an unreceived board delivery for a thread
- **THEN** the thread's unread count includes that delivery

#### Scenario: Fresh and stale requests
- **WHEN** an identity has requests in a thread that remain unanswered, uncancelled, and nonsuperseded
- **THEN** fresh counts match default `tala pending` and stale counts match `tala pending --stale`, using the existing 24-hour and inactive-counterpart rule

#### Scenario: Listing does not consume messages
- **WHEN** an identity lists threads with unread direct messages or board deliveries
- **THEN** their receipt state remains unchanged and inbox or board reads still return those deliveries as unread
