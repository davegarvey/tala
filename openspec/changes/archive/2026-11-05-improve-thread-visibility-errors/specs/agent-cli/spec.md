# Spec Delta

## MODIFIED Requirements

### Requirement: Actionable errors
Every error SHALL include `code`, `message`, `hint`, a `retry` class of `safe`, `same_key`, `after_fix` or `never`, a `stored` message ID when a message was stored before the failure (otherwise null), and a `next` field holding one exact command or null. The set of error codes SHALL be documented in help output.

#### Scenario: Transport failure before storage
- **WHEN** a send fails with `CONNECTION_ERROR` before the daemon confirms storage
- **THEN** the error has `retry` of `same_key`, `stored` null, and a `next` command that repeats the send with its key

#### Scenario: Daemon unreachable
- **WHEN** a command fails with `DAEMON_UNREACHABLE`
- **THEN** the error has `retry` of `same_key`, `stored` null, and `next` of `tala status`

#### Scenario: Invalid input
- **WHEN** a send names an unknown recipient
- **THEN** the error has `retry` of `after_fix` and `next` lists agents

#### Scenario: Thread outside the visible scope
- **WHEN** an agent accesses a thread it cannot see through its selected identity and project context
- **THEN** the error uses the generic `THREAD_NOT_VISIBLE` code and does not disclose whether the thread exists or who owns it
- **AND** its hint explains safe recovery through the original private identity or the thread owner, without exposing handles or foreign project metadata
- **AND** its `next` command is `tala guide identity`
