## Purpose

Retry-safe message delivery in tala: every send carries a client-generated idempotency key, and the daemon deduplicates on it, so a timed-out or retried send can never double-post a message.

## Requirements

### Requirement: Idempotency key on sends

Every send request to the daemon SHALL carry an `idempotency_key` string. The CLI SHALL generate a fresh random key once per `tala send` invocation and SHALL reuse that same key for every retry of that send (connection failure, lost response, daemon restart). When a send POST fails with a connection error, the CLI SHALL retry it with the same key, up to two additional attempts. A send without an idempotency key SHALL be rejected by the daemon with an error.

#### Scenario: Key reused across retries
- **WHEN** a user runs `tala send --wait "help"` and the daemon's connection fails after the request may have been received (daemon restarted, response lost)
- **THEN** the CLI SHALL retry the POST using the same idempotency key it generated at invocation start
- **AND** the daemon SHALL store at most one message

#### Scenario: Missing key rejected
- **WHEN** a client posts a message to the daemon without an `idempotency_key`
- **THEN** the daemon SHALL reject the request with an error and SHALL NOT store a message

### Requirement: Daemon-side deduplication

The daemon SHALL record the idempotency key per sender. When a send arrives with a key already recorded for that sender, the daemon SHALL NOT store a new message and SHALL NOT emit a new-message event; it SHALL report the original message that the key was first recorded with. When a send arrives with a key already recorded for that sender but with different content, the daemon SHALL reject it with an error identifying the key conflict.

#### Scenario: Retry does not duplicate
- **WHEN** a client sends a message with key `k1` and the request times out before the response returns
- **AND** the client retries with the same key `k1`
- **THEN** exactly one message SHALL exist in the session
- **AND** the retry response SHALL identify the original stored message

#### Scenario: Same key with different content rejected
- **WHEN** a client sends a message with key `k1`, then sends a different message with the same key `k1`
- **THEN** the second send SHALL fail with an error naming the key conflict
- **AND** the second message SHALL NOT be stored

### Requirement: Replay reporting

When a send is deduplicated, the CLI SHALL report that the message was already stored and identify the original message id and session (e.g. "duplicate suppressed (msg 12)"). The command SHALL exit successfully. In `--json` mode the duplicate SHALL be reported as typed fields within the command's normal JSON output (e.g. `duplicate: true`, the original message id, and its session id), with no additional stdout text.

#### Scenario: CLI reports deduplicated replay
- **WHEN** a client retries a send and the daemon returns the original message id 12
- **THEN** the CLI SHALL print a note that the message was already stored, naming message 12
- **AND** the command SHALL exit with status 0

#### Scenario: JSON mode reports duplicate as typed fields
- **WHEN** a client runs `tala send --json` and the daemon deduplicates against the original message
- **THEN** the emitted JSON SHALL contain `duplicate: true` with the original message id and session id
- **AND** no additional text SHALL be written to stdout
