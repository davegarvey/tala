# Spec Delta

## MODIFIED Requirements

### Requirement: Exit codes
The CLI SHALL exit 0 on success, 1 on operational error with nothing stored, 2 on usage error, 3 on wait timeout and 4 when a message was stored but a subsequent wait failed. Help output SHALL list these codes. When `send --wait` times out after storing its request, its result SHALL identify the stored request and provide one exact command to resume waiting for that request without resending it.

#### Scenario: Stored then failed
- **WHEN** `send --wait` stores its request and the wait then fails permanently
- **THEN** exit is 4 and the error's `stored` field holds the message ID

#### Scenario: Stored request times out
- **WHEN** `send --wait --timeout=N` stores its request but receives no correlated reply before the deadline
- **THEN** it exits 3, reports that the request is stored and timed out, and provides a `next` command using `inbox --wait --reply-to=<message-id> --timeout=N`
- **AND** the command does not suggest resending the request
