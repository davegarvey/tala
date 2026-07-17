## ADDED Requirements

### Requirement: `chit observe --timeout` terminates after given seconds
`chit observe` SHALL accept a `--timeout` argument (in seconds). When provided, the observe stream SHALL terminate after the specified duration, even if no messages have been received. When not provided, the stream SHALL run indefinitely (current behavior). A value of `0` SHALL be treated as equivalent to omitting the flag (no timeout).

#### Scenario: Observe with timeout exits cleanly
- **WHEN** user runs `chit observe --timeout 3`
- **THEN** the SSE stream SHALL run for approximately 3 seconds
- **THEN** the command SHALL exit with success
- **THEN** any messages received during the 3 seconds SHALL be printed

#### Scenario: Observe without timeout runs indefinitely
- **WHEN** user runs `chit observe` without `--timeout`
- **THEN** the SSE stream SHALL continue until the user interrupts (Ctrl+C)

#### Scenario: Observe with JSON + timeout
- **WHEN** user runs `chit observe --json --timeout 5`
- **THEN** JSON lines SHALL be printed for 5 seconds
- **THEN** the command SHALL exit with success

#### Scenario: Observe with timeout 0 is same as no timeout
- **WHEN** user runs `chit observe --timeout 0`
- **THEN** the SSE stream SHALL run indefinitely (same as without `--timeout`)

### Requirement: `chit observe` timeout is server-driven
The timeout SHALL be implemented server-side in the `/api/observe` SSE endpoint, not client-side. The client SHALL pass `timeout_secs` as a query parameter. The server SHALL close the SSE stream after the timeout by using `tokio::select!` to race the broadcast receiver against a sleep timer.

#### Scenario: Timeout stops SSE stream
- **WHEN** the daemon receives `GET /api/observe?timeout_secs=3`
- **THEN** the SSE stream SHALL close after 3 seconds
- **THEN** the HTTP response SHALL complete normally

#### Scenario: Multiple concurrent observe with timeout
- **WHEN** two clients connect with `GET /api/observe?timeout_secs=5`
- **THEN** both SHALL receive events independently
- **THEN** both SHALL disconnect after approximately 5 seconds
