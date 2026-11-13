# Spec Delta

## ADDED Requirements

### Requirement: Overdue delivery alerts reflect recipient state
The system SHALL emit at most one delivery-overdue event per unread message, and SHALL not emit the event when its recipient is inactive when the overdue threshold is processed. Send output SHALL distinguish the persisted send-time listening observation from the recipient's current active/listening state.

#### Scenario: Overdue message alerts once
- **WHEN** an unread message passes its delivery-overdue threshold and its recipient is active
- **THEN** the sender receives one delivery-overdue event even if the wait snapshot is replayed

#### Scenario: Inactive recipient does not trigger overdue wakeup
- **WHEN** an unread message passes its threshold while its recipient is inactive
- **THEN** the sender receives no delivery-overdue event and the send response's current state identifies the recipient as inactive

#### Scenario: Active recipient is not listening
- **WHEN** a message is sent to an active recipient that is not currently listening
- **THEN** the send response's current state identifies the recipient as active but not listening
