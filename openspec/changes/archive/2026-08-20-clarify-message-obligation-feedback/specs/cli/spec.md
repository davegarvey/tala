## ADDED Requirements

### Requirement: Blocking send identifies received replies

When `tala send --wait` receives one or more messages, human-readable output SHALL identify each received message by its message id and sender. When a received message has intent or reply-correlation metadata, the output SHALL render that metadata using the same visible intent/correlation convention as other message surfaces. The command's `--json` output SHALL retain the existing structured message identifiers and correlation fields and SHALL remain valid machine-readable output.

#### Scenario: Blocking send shows the received message id

- **WHEN** message `1` is sent with `tala send --wait` and message `2` is received as its correlated reply
- **THEN** human-readable output SHALL include message id `2`, the replying sender, and the reply correlation to message `1`
- **AND** the output SHALL make clear which received message ended the wait

#### Scenario: Blocking send with multiple received messages

- **WHEN** `tala send --wait` receives multiple messages before the wait completes
- **THEN** each displayed received message SHALL include its own message id and sender
- **AND** any available intent or reply-correlation metadata SHALL remain visible for each message

#### Scenario: JSON blocking-send output remains structured

- **WHEN** `tala send --wait --json` receives a reply
- **THEN** stdout SHALL remain valid JSON according to the existing wait response contract
- **AND** the received message id and `reply_to` value SHALL be available as structured fields rather than only in rendered text
