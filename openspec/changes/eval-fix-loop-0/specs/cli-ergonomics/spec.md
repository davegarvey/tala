## ADDED Requirements

### Requirement: Stream command replaces Watch
`tala stream` SHALL be the canonical name for SSE message streaming. `tala watch` SHALL remain as a deprecated hidden alias that prints a deprecation warning before delegating.

#### Scenario: stream is the primary command
- **WHEN** user runs `tala stream --help`
- **THEN** the help text SHALL show `stream` as the command name with description "Stream new messages as they arrive (SSE)"

#### Scenario: watch still works with deprecation warning
- **WHEN** user runs `tala watch`
- **THEN** it SHALL function identically to `tala stream` AND SHALL print a deprecation warning to stderr

#### Scenario: watch is hidden from help
- **WHEN** user runs `tala --help`
- **THEN** `watch` SHALL NOT appear in the command listing; `stream` SHALL appear instead

### Requirement: Stream non-empty on timeout
`tala stream` SHALL always produce output when it exits, even if no messages were received during the session.

#### Scenario: timeout with no messages in text mode
- **WHEN** user runs `tala stream --timeout 3 --json`
- **THEN** the command SHALL output `[]` before exiting

#### Scenario: timeout with no messages in text mode
- **WHEN** user runs `tala stream --timeout 3`
- **THEN** the command SHALL print `[no messages received]` to stdout before exiting

#### Scenario: messages received during stream
- **WHEN** messages are received during `tala stream`
- **THEN** no trailing empty notice SHALL be printed

### Requirement: Send --wait progress indicator
`tala send --wait` SHALL show progress indication while waiting for a reply during the long-poll.

#### Scenario: heartbeat dots during wait
- **WHEN** user runs `tala send "hello" --wait`
- **THEN** the command SHALL print heartbeat dots (`.`) to stderr at regular intervals until the reply arrives or timeout occurs

#### Scenario: JSON mode no heartbeat
- **WHEN** user runs `tala send "hello" --wait --json`
- **THEN** no heartbeat dots SHALL be printed to stderr

### Requirement: Observe deprecation visibility
`tala observe` deprecation warning SHALL be more prominent to steer users to `tala listen`.

#### Scenario: observe warning on stderr
- **WHEN** user runs `tala observe`
- **THEN** a deprecation warning SHALL appear on stderr indicating `observe` is deprecated and `tala listen` should be used

### Requirement: Listen help text clarity
`tala listen` help text SHALL clearly indicate it watches all sessions (not a single session).

#### Scenario: listen description mentions all sessions
- **WHEN** user runs `tala listen --help`
- **THEN** the description SHALL mention "all sessions"

### Requirement: Send missing stdin error hint
When no message is provided and `--stdin` is not passed, `tala send` SHALL mention `--stdin` in the error message.

#### Scenario: error suggests --stdin
- **WHEN** user runs `tala send` with no message, no `--file`, no piped input, and no `--stdin`
- **THEN** the error message SHALL mention `--stdin` as a way to read from stdin
