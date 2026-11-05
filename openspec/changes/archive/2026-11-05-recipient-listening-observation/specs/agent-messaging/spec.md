## ADDED Requirements

### Requirement: Direct-message recipient listening observation
Direct send and direct reply results SHALL include `recipient_presence.listening`, a boolean captured from the resolved recipient's active listening lease when Tala accepts the message. This is a point-in-time observation and SHALL NOT imply that the recipient will respond. Matching idempotent retries SHALL return the original observation. Board posts and board replies SHALL omit this field.

#### Scenario: Recipient is listening
- **WHEN** a direct message is accepted while its active recipient has a current listening lease
- **THEN** the result includes `recipient_presence.listening: true`

#### Scenario: Recipient is not listening
- **WHEN** a direct message is accepted while its recipient is inactive or has no current listening lease
- **THEN** the result includes `recipient_presence.listening: false` and the message remains queued for later retrieval

#### Scenario: Direct reply observes original sender
- **WHEN** a direct reply is accepted
- **THEN** its presence observation describes the original sender, who is the reply recipient

#### Scenario: Idempotent retry preserves observation
- **WHEN** an identical direct send or reply is retried after the recipient's listening state changes
- **THEN** the result returns the original message and original `recipient_presence.listening` value

#### Scenario: Board activity has no recipient presence
- **WHEN** a board post or board reply is accepted
- **THEN** the result omits `recipient_presence`
