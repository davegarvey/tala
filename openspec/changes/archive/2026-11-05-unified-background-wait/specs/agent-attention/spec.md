### Requirement: Background wait notifications do not acknowledge
`tala wait` SHALL monitor unread inbox messages using a non-consuming read, including direct replies and board replies delivered to the agent. It SHALL advance its inbox notification cursor only through events returned to the caller and SHALL leave inbox receipt state unchanged. Continuation commands SHALL carry separate inbox and board cursors; replaying an older inbox cursor MAY repeat an event but SHALL NOT hide it or mark it received.

#### Scenario: Wait notification preserves unread receipt
- **WHEN** `tala wait` returns an unread message notification
- **THEN** `tala inbox --peek` still returns that message as unread

#### Scenario: Lost wait output can safely replay
- **WHEN** an agent resumes with an earlier inbox cursor after losing a wait result
- **THEN** the notification may repeat
- **AND** the message remains available until explicitly consumed
