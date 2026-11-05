## ADDED Requirements

### Requirement: Human recipient presence wording
Human-readable output for direct sends and direct replies SHALL describe recipient presence as a point-in-time observation. When the observation says the recipient was not listening, output SHALL state that the message remains queued.

#### Scenario: Direct message recipient is not listening
- **WHEN** a direct send or direct reply is rendered with `--human` and `recipient_presence.listening` is false
- **THEN** output says the recipient was not listening and the message remains queued

#### Scenario: Blocking send includes recipient observation
- **WHEN** a direct `send --wait` result is rendered with `--human`
- **THEN** output includes the observation from the stored send result, including when the correlated reply wait times out
